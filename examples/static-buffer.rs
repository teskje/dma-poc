//! This example demonstrates performing a DMA read into a static buffer.

#![no_std]
#![no_main]

use cortex_m::singleton;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    let transfer = start();
    let (_dma, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(dst, SRC);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<&'static mut [u8]> {
    let dst = singleton!(: [u8; 16] = [0; 16]).unwrap();
    Transfer::start(SRC, dst)
}
