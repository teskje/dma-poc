//! This example demonstrates performing a DMA read into a stack buffer.

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

#[entry]
fn main() -> ! {
    let src = b"THIS IS DMADATA!";
    let mut dst = [0; 16];

    // Note: This is only safe as long as we don't `mem::forget` the transfer.
    let transfer = unsafe { Transfer::start_nonstatic(src, &mut dst) };
    let (_dma, src, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(src, dst);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}
