use as_slice::AsSlice;
use core::{
    mem,
    ops::{Deref, DerefMut},
};
use stable_deref_trait::StableDeref;

pub unsafe trait DmaReadBuffer: Deref {
    fn dma_read_buffer(&self) -> (*const u8, usize);
}

pub unsafe trait DmaWriteBuffer: DerefMut {
    fn dma_write_buffer(&mut self) -> (*mut u8, usize);
}

unsafe impl<B, W> DmaReadBuffer for B
where
    B: Deref + StableDeref,
    B::Target: AsSlice<Element = W>,
{
    fn dma_read_buffer(&self) -> (*const u8, usize) {
        let slice = self.as_slice();
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * mem::size_of::<W>();
        (ptr, len)
    }
}

pub unsafe trait DmaWriteTarget {}

unsafe impl DmaWriteTarget for u8 {}
unsafe impl DmaWriteTarget for u16 {}
unsafe impl DmaWriteTarget for u32 {}
unsafe impl DmaWriteTarget for [u8] {}
unsafe impl DmaWriteTarget for [u16] {}
unsafe impl DmaWriteTarget for [u32] {}

macro_rules! array_impls {
    ( $( $i:expr, )+ ) => {
        $(
            unsafe impl DmaWriteTarget for [u8; $i] {}
            unsafe impl DmaWriteTarget for [u16; $i] {}
            unsafe impl DmaWriteTarget for [u32; $i] {}
        )+
    };
}

#[rustfmt::skip]
array_impls!(
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9,
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
    30, 31, 32,
);

unsafe impl<B, T> DmaWriteBuffer for B
where
    B: DerefMut<Target = T> + StableDeref,
    T: DmaWriteTarget + ?Sized,
{
    fn dma_write_buffer(&mut self) -> (*mut u8, usize) {
        let target = self.deref_mut();
        let ptr = target as *mut T as *mut u8;
        let len = mem::size_of_val(target);
        (ptr, len)
    }
}
