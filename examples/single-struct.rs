//! This example demonstrates performing a DMA read into a singel struct
//! (no array/slice involved).

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use dma_poc::Transfer;
use zerocopy::FromBytes;

#[derive(Debug, PartialEq, FromBytes)]
struct Message {
    foo: u32,
    bar: u16,
    baz: [u8; 5],
}

const SRC: Message = Message {
    foo: 100,
    bar: 42,
    baz: *b"hello",
};

static mut DST: Message = Message {
    foo: 0,
    bar: 0,
    baz: [0; 5],
};

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
fn start() -> Transfer<&'static Message, &'static mut Message> {
    let dst = unsafe { &mut DST };
    Transfer::start(&SRC, dst)
}
