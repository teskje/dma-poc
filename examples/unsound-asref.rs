//! This is a PoC that shows that using `AsRef` (or `AsSlice`) as a bound
//! for the `B::Target` of a DMA write is unsafe.
//!
//! The PoC uses `AsRef` rather than `AsSlice` because it is easier to
//! implement that way (`heapless::String` doesn't implement `AsSlice`).
//! However, the issue also applies to `AsSlice`.

#![no_std]
#![no_main]

use core::{
    ops::DerefMut,
    str,
    sync::atomic::{self, Ordering},
};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Dma;
use heapless::{consts::*, String};
use stable_deref_trait::StableDeref;

/// Transfer implementation that bounds `B::Target` to `AsRef<[u8]>`.
///
/// Note: Left out the `Drop` impl for simplicity, it wouldn't help here.
pub struct Transfer<B> {
    dma: Dma,
    buffer: B,
}

impl<B> Transfer<B> {
    pub fn start(src: &'static [u8], dst: B) -> Self
    where
        B: DerefMut + StableDeref + 'static,
        B::Target: AsRef<[u8]>,
    {
        let slice = dst.as_ref();

        let mut dma = Dma::mem2mem();
        dma.set_paddr(src.as_ptr() as u32);
        dma.set_maddr(slice.as_ptr() as u32);
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

const SRC: &[u8; 16] = b"invalid utf8: \xc3\x28";
static mut DST: String<U16> = String(heapless::i::String::new());

#[entry]
fn main() -> ! {
    let transfer = start();
    let (_dma, dst) = transfer.wait().expect("Transfer error");

    // this panics
    str::from_utf8(dst.as_ref()).expect("invalid data in String");

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<&'static mut String<U16>> {
    let dst = unsafe { &mut DST };
    for _ in 0..16 {
        dst.push('\x00').unwrap();
    }

    Transfer::start(SRC, dst)
}
