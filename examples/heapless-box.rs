//! This example demonstrates performing a DMA read into a heapless `Box`.

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;
use heapless::{
    pool,
    pool::singleton::{Box, Pool},
};

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

pool!(P: [u8; 16]);

#[entry]
fn main() -> ! {
    static mut MEMORY: [u8; 1024] = [0; 1024];
    P::grow(MEMORY);

    let transfer = start();
    let (_dma, src, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(src, *dst);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<&'static [u8], Box<P>> {
    let dst = P::alloc().unwrap().freeze();
    Transfer::start(SRC, dst)
}
