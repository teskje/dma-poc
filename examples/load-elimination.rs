//! This example shows that the DMA abstraction is not made unsound by
//! LLVM's load elimination.
//!
//! Note: This works only thanks to the compiler fences in `Transfer`'s
//! methods. If they are removed, the `assert_eq` will fail.
//!
//! Here is a smaller self-contained example of how fences prevent load
//! elimination (look at the generated LLVM IR):
//! https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=3d71954aa7ced56b5507f467a32c4c0b

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;

const SRC: &[u8; 16] = b"THIS IS DMADATA!";

#[entry]
fn main() -> ! {
    let mut dst = [0; 16];

    let x = b'X';
    dst[8] = x;

    let transfer = unsafe { Transfer::start_nonstatic(SRC, &mut dst) };
    let (_dma, _src, dst) = transfer.wait().expect("Transfer error");

    // If the compiler eliminated this load and used the known value 'X'
    // instead, this assert would fail.
    assert_eq!(dst[8], b'D');

    hprintln!("Transfer finished successfully").unwrap();
    loop {
        continue;
    }
}
