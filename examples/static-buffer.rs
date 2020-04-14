//! This example demonstrates performing a DMA read into a static buffer.

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

const SRC: &[u8; 16] = b"THIS IS DMADATA!";
static mut DST: [u8; 16] = [0; 16];

#[entry]
fn main() -> ! {
    let transfer = start();
    let (_dma, src, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(src, dst);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}

#[inline(never)]
fn start() -> Transfer<&'static [u8], &'static mut [u8]> {
    let dst = unsafe { &mut DST };
    Transfer::start(SRC, dst)
}
