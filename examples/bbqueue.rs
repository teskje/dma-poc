//! This example demonstrates performing a DMA read into bbqueue Grant.

#![no_std]
#![no_main]

use bbqueue::{consts::*, BBBuffer, ConstBBBuffer};
use core::ops::{Deref, DerefMut};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;
use stable_deref_trait::StableDeref;

// Since bbqueue's grant types don't (yet) implement `StableDeref`,
// we need to wrap them here to add that ourselves.

struct R(bbqueue::GrantR<'static, U32>);

impl Deref for R {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

unsafe impl StableDeref for R {}

struct W(bbqueue::GrantW<'static, U32>);

impl Deref for W {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

impl DerefMut for W {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut *self.0
    }
}

unsafe impl StableDeref for W {}

static BB: BBBuffer<U32> = BBBuffer(ConstBBBuffer::new());

#[entry]
fn main() -> ! {
    let (mut prod, mut cons) = BB.try_split().unwrap();

    // prepare the src
    let mut wgr = prod.grant_exact(16).unwrap();
    wgr.copy_from_slice(b"THIS IS DMADATA!");
    wgr.commit(16);

    let src = cons.read().unwrap();
    let dst = prod.grant_exact(16).unwrap();

    let transfer = Transfer::start(R(src), W(dst));
    let (_dma, src, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(*src, *dst);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}
