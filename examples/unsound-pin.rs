//! This is a PoC that shows that using `Pin` is not sufficient to ensure
//! a DMA buffer is fixed in memory.

#![no_std]
#![no_main]

use as_slice::AsMutSlice;
use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{self, Ordering},
};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Dma;

/// Transfer implementation that attempts to use `Pin` instead of
/// `StableDeref` to ensure the DMA buffer is stable in memory.
///
/// Note: Left out the `Drop` impl for simplicity, it wouldn't help here.
pub struct Transfer<B> {
    dma: Dma,
    buffer: Pin<B>,
}

impl<B> Transfer<B> {
    pub fn start(src: &'static [u8], mut dst: Pin<B>) -> Self
    where
        B: DerefMut + 'static,
        B::Target: AsMutSlice<Element = u8> + Unpin,
    {
        let slice = dst.as_mut_slice();

        let mut dma = Dma::mem2mem();
        dma.set_paddr(src.as_ptr() as u32);
        dma.set_maddr(slice.as_mut_ptr() as u32);
        dma.set_ndt(slice.len() as u16);

        atomic::compiler_fence(Ordering::Release);
        dma.enable();

        Transfer { dma, buffer: dst }
    }

    pub fn wait(mut self) -> Result<(Dma, Pin<B>), ()> {
        while !self.dma.transfer_complete() {
            if self.dma.transfer_error() {
                return Err(());
            }
        }
        atomic::compiler_fence(Ordering::Acquire);

        self.dma.disable();

        Ok((self.dma, self.buffer))
    }
}

/// Using this buffer with DMA is unsafe since its allocated on the stack
/// and will therefore move around. `Pin` doesn't prevent us from using
/// this buffer type.
#[derive(Debug)]
struct Buffer([u8; 16]);

impl Deref for Buffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    let transfer = start();
    let (_dma, dst) = transfer.wait().expect("Transfer error");

    // this panics
    assert_eq!(*dst, *SRC);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<Buffer> {
    let dst = Buffer([0; 16]);
    Transfer::start(SRC, Pin::new(dst))
}
