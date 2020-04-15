#![no_std]

use panic_semihosting as _;

use as_slice::{AsMutSlice, AsSlice};
use core::{
    ops::{Deref, DerefMut},
    sync::atomic::{self, Ordering},
};
use stable_deref_trait::StableDeref;
use stm32f3::stm32f303 as pac;

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

/// Safe (?) abstraction of a DMA read transfer.
pub struct Transfer<A, B> {
    // always `Some` outside of `Drop::drop`
    inner: Option<TransferInner<A, B>>,
}

impl<A, B> Transfer<A, B> {
    pub fn start(src: A, dst: B) -> Self
    where
        A: Deref + StableDeref + 'static,
        A::Target: AsSlice<Element = u8>,
        B: DerefMut + StableDeref + 'static,
        B::Target: AsMutSlice<Element = u8>, // FIXME: this is unsafe
    {
        unsafe { Self::start_nonstatic(src, dst) }
    }

    /// # Safety
    ///
    /// If `dst` is not `'static`, callers must ensure that `mem::forget`
    /// is never called on the returned `Transfer`.
    pub unsafe fn start_nonstatic(src: A, mut dst: B) -> Self
    where
        A: Deref + StableDeref,
        A::Target: AsSlice<Element = u8>,
        B: DerefMut + StableDeref,
        B::Target: AsMutSlice<Element = u8>, // FIXME: this is unsafe
    {
        let mut dma = Dma::mem2mem();
        {
            let src = src.as_slice();
            let dst = dst.as_mut_slice();
            assert!(dst.len() >= src.len());

            dma.set_paddr(src.as_ptr() as u32);
            dma.set_maddr(dst.as_mut_ptr() as u32);
            dma.set_ndt(src.len() as u16);
        }

        // Prevent preceding reads/writes on the buffer from being moved past
        // the DMA enable modify (i.e. after the transfer has started).
        atomic::compiler_fence(Ordering::Release);

        dma.enable();

        Transfer {
            inner: Some(TransferInner { dma, src, dst }),
        }
    }

    pub fn wait(mut self) -> Result<(Dma, A, B), ()> {
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

struct TransferInner<A, B> {
    dma: Dma,
    src: A,
    dst: B,
}

impl<A, B> TransferInner<A, B> {
    fn stop(&mut self) {
        self.dma.disable();

        // Prevent subsequent reads/writes on the buffer from being moved
        // ahead of the DMA disable modify (i.e. before the transfer is
        // stopped).
        atomic::compiler_fence(Ordering::Acquire);
    }
}

impl<A, B> Drop for Transfer<A, B> {
    fn drop(&mut self) {
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }
}
