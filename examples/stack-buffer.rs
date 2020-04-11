//! This example demonstrates performing a DMA read into a stack buffer.

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    let mut dst = [0; 16];

    // Note: This is only safe as long as we don't `mem::forget` the transfer.
    let transfer = unsafe { Transfer::start_nonstatic(SRC, &mut dst) };
    let (_dma, dst) = transfer.wait().expect("Transfer error");

    assert_eq!(dst, SRC);

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}
