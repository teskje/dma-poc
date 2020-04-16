//! This is a PoC that shows that using a non-'static buffer is unsafe in the
//! face of `mem::forget`.

#![no_std]
#![no_main]

use core::mem;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    corrupt_stack();
    use_stack();

    loop {
        continue;
    }
}

#[inline(never)]
fn corrupt_stack() {
    let mut dst = [0_u8; 16];

    // for some reason necessary to trigger the panic
    hprintln!("{}", dst[0]).unwrap();

    let transfer = unsafe { Transfer::start_nonstatic(SRC, &mut dst) };
    mem::forget(transfer);

    // `dst` gets freed here, but the DMA transfer continues writing to it.
}

#[inline(never)]
fn use_stack() {
    let buf = [0_u8; 16];

    // this panics
    assert_eq!(buf, [0; 16]);
}
