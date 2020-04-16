use as_slice::AsSlice;
use core::ops::{Deref, DerefMut};
use stable_deref_trait::StableDeref;

pub unsafe trait DmaReadBuffer: Deref {
    type Element;
    fn dma_read_buffer(&self) -> (*const Self::Element, usize);
}

pub unsafe trait DmaWriteBuffer: DerefMut {
    type Element;
    fn dma_write_buffer(&mut self) -> (*mut Self::Element, usize);
}

unsafe impl<B, W> DmaReadBuffer for B
where
    B: Deref + StableDeref,
    B::Target: AsSlice<Element = W>,
{
    type Element = W;
    fn dma_read_buffer(&self) -> (*const Self::Element, usize) {
        let slice = self.as_slice();
        let ptr = slice.as_ptr() as *const Self::Element;
        let len = slice.len();
        (ptr, len)
    }
}

pub unsafe trait DmaWriteTarget {
    type Element;
    fn len(&self) -> usize;
}

unsafe impl DmaWriteTarget for u8 {
    type Element = u8;
    fn len(&self) -> usize {
        1
    }
}
unsafe impl DmaWriteTarget for u16 {
    type Element = u16;
    fn len(&self) -> usize {
        1
    }
}
unsafe impl DmaWriteTarget for u32 {
    type Element = u32;
    fn len(&self) -> usize {
        1
    }
}
unsafe impl DmaWriteTarget for [u8] {
    type Element = u8;
    fn len(&self) -> usize {
        self.len()
    }
}
unsafe impl DmaWriteTarget for [u16] {
    type Element = u16;
    fn len(&self) -> usize {
        self.len()
    }
}
unsafe impl DmaWriteTarget for [u32] {
    type Element = u32;
    fn len(&self) -> usize {
        self.len()
    }
}

macro_rules! array_impls {
    ( $( $i:expr, )+ ) => {
        $(
            unsafe impl DmaWriteTarget for [u8; $i] {
                type Element = u8;
                fn len(&self) -> usize {
                    $i
                }
            }
            unsafe impl DmaWriteTarget for [u16; $i] {
                type Element = u16;
                fn len(&self) -> usize {
                    $i
                }
            }
            unsafe impl DmaWriteTarget for [u32; $i] {
                type Element = u32;
                fn len(&self) -> usize {
                    $i
                }
            }
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
    type Element = <T as DmaWriteTarget>::Element;
    fn dma_write_buffer(&mut self) -> (*mut Self::Element, usize) {
        let target = self.deref_mut();
        let len = target.len();
        let ptr = target as *mut T as *mut Self::Element;
        (ptr, len)
    }
}
