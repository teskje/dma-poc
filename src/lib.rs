#![no_std]

mod traits;

use panic_semihosting as _;

use core::sync::atomic::{self, Ordering};
use stm32f3::stm32f303 as pac;

pub use traits::{DmaReadBuffer, DmaWriteBuffer};

/// Thin wrapper around the DMA1 peripheral, using channel 1.
pub struct Dma(pac::DMA1);

impl Dma {
    pub fn mem2mem() -> Self {
        let device = pac::Peripherals::take().unwrap();

        // enable DMA1 peripheral
        device.RCC.ahbenr.modify(|_, w| w.dma1en().enabled());

        // setup channel 1 for mem2mem transfer
        let dma1 = device.DMA1;
        dma1.ch1.cr.write(|w| {
            w.dir().from_peripheral();
            w.pinc().enabled();
            w.minc().enabled();
            w.psize().bits8();
            w.msize().bits8();
            w.mem2mem().enabled()
        });

        Self(dma1)
    }

    pub fn set_paddr(&mut self, addr: u32) {
        self.0.ch1.par.write(|w| w.pa().bits(addr));
    }

    pub fn set_maddr(&mut self, addr: u32) {
        self.0.ch1.mar.write(|w| w.ma().bits(addr));
    }

    pub fn set_ndt(&mut self, len: u16) {
        self.0.ch1.ndtr.write(|w| w.ndt().bits(len));
    }

    pub fn enable(&mut self) {
        // clear interrupt flags
        self.0.ifcr.write(|w| w.cgif1().set_bit());

        self.0.ch1.cr.modify(|_, w| w.en().enabled());
    }

    pub fn disable(&mut self) {
        self.0.ch1.cr.modify(|_, w| w.en().disabled());
    }

    pub fn transfer_complete(&self) -> bool {
        self.0.isr.read().tcif1().bit_is_set()
    }

    pub fn transfer_error(&self) -> bool {
        self.0.isr.read().teif1().bit_is_set()
    }
}

/// Safe abstraction of a DMA read transfer.
pub struct Transfer<R, W> {
    // always `Some` outside of `Drop::drop`
    inner: Option<TransferInner<R, W>>,
}

impl<R, W> Transfer<R, W> {
    pub fn start(src: R, dst: W) -> Self
    where
        R: DmaReadBuffer + 'static,
        W: DmaWriteBuffer + 'static,
    {
        unsafe { Self::start_nonstatic(src, dst) }
    }

    /// # Safety
    ///
    /// If `dst` is not `'static`, callers must ensure that `mem::forget`
    /// is never called on the returned `Transfer`.
    pub unsafe fn start_nonstatic(src: R, mut dst: W) -> Self
    where
        R: DmaReadBuffer,
        W: DmaWriteBuffer,
    {
        let mut dma = Dma::mem2mem();
        {
            let (src_ptr, src_len) = src.dma_read_buffer();
            let (dst_ptr, dst_len) = dst.dma_write_buffer();
            assert!(dst_len >= src_len);

            dma.set_paddr(src_ptr as *const u8 as u32);
            dma.set_maddr(dst_ptr as *mut u8 as u32);
            dma.set_ndt(src_len as u16);
        }

        // Prevent preceding reads/writes on the buffer from being moved past
        // the DMA enable modify (i.e. after the transfer has started).
        atomic::compiler_fence(Ordering::Release);

        dma.enable();

        Transfer {
            inner: Some(TransferInner { dma, src, dst }),
        }
    }

    pub fn wait(mut self) -> Result<(Dma, R, W), ()> {
        let mut inner = self.inner.take().unwrap();

        while !inner.dma.transfer_complete() {
            if inner.dma.transfer_error() {
                return Err(());
            }
        }

        inner.stop();

        Ok((inner.dma, inner.src, inner.dst))
    }
}

struct TransferInner<R, W> {
    dma: Dma,
    src: R,
    dst: W,
}

impl<R, W> TransferInner<R, W> {
    fn stop(&mut self) {
        self.dma.disable();

        // Prevent subsequent reads/writes on the buffer from being moved
        // ahead of the DMA disable modify (i.e. before the transfer is
        // stopped).
        atomic::compiler_fence(Ordering::Acquire);
    }
}

impl<R, W> Drop for Transfer<R, W> {
    fn drop(&mut self) {
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }
}
