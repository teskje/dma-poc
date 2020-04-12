//! This is a PoC that shows that using a non-pointer buffer is unsafe.

#![no_std]
#![no_main]

use as_slice::AsMutSlice;
use core::sync::atomic::{self, Ordering};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Dma;

/// Transfer implementation that doesn't restrict `B` to be a pointer type.
///
/// Note: Left out the `Drop` impl for simplicity, it wouldn't help here.
pub struct Transfer<B> {
    dma: Dma,
    buffer: B,
}

impl<B> Transfer<B> {
    pub fn start(src: &'static [u8], mut dst: B) -> Self
    where
        B: AsMutSlice<Element = u8>,
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

    pub fn wait(mut self) -> Result<(Dma, B), ()> {
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

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    let transfer = start();
    let (_dma, dst) = transfer.wait().expect("Transfer error");

    // this panics
    assert_eq!(dst, *SRC);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<[u8; 16]> {
    let dst = [0; 16];
    Transfer::start(SRC, dst)
}
