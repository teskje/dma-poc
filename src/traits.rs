//! `unsafe` traits for buffers usable with DMA.
//!
//! The traits defined here are concerned with ensuring requirements
//! 1, 2, and 4 from the README. They ignore requirement 3 (`'static` bound)
//! to make them useful for DMA on stack buffers too. Requirement 3 must be
//! enforced by the `Transfer` implementation instead.

use as_slice::AsSlice;
use core::{
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut},
};
use stable_deref_trait::StableDeref;

/// Trait for buffers that can be given to DMA for reading.
///
/// # Safety
///
/// The implementing type must be safe to use for DMA reads. This means:
///
/// - It must be a pointer that references the actual buffer.
/// - The requirements documented on `dma_read_buffer` must be fulfilled.
pub unsafe trait DmaReadBuffer {
    type Target: ?Sized;

    /// Provide a buffer usable for DMA reads.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in bytes
    ///
    /// # Safety
    ///
    /// - This function must always return the same values, if called multiple
    ///   times.
    /// - The memory specified by the returned pointer and size must be fully
    ///   readable by the DMA peripheral.
    fn dma_read_buffer(&self) -> (*const Self::Target, usize);
}

/// Trait for buffers that can be given to DMA for writing.
///
/// # Safety
///
/// The implementing type must be safe to use for DMA writes. This means:
///
/// - It must be a pointer that references the actual buffer.
/// - `Target` must be a type that is valid for any possible byte pattern.
/// - The requirements documented on `dma_write_buffer` must be fulfilled.
pub unsafe trait DmaWriteBuffer {
    type Target: ?Sized;

    /// Provide a buffer usable for DMA writes.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in bytes
    ///
    /// # Safety
    ///
    /// - This function must always return the same values, if called multiple
    ///   times.
    /// - The memory specified by the returned pointer and size must be fully
    ///   writable by the DMA peripheral.
    fn dma_write_buffer(&mut self) -> (*mut Self::Target, usize);
}

/// Deref target for the `Dma{Read,Write}Buffer` types used by the blanket
/// implementations.
///
/// # Safety
///
/// Types that implement this trait must be valid for every possible byte
/// pattern. This is to ensure that, whatever DMA writes into the buffer,
/// we won't get UB due to invalid values.
pub unsafe trait DmaTarget {}

unsafe impl DmaTarget for u8 {}
unsafe impl DmaTarget for u16 {}
unsafe impl DmaTarget for u32 {}
unsafe impl DmaTarget for u64 {}
unsafe impl DmaTarget for usize {}

unsafe impl DmaTarget for i8 {}
unsafe impl DmaTarget for i16 {}
unsafe impl DmaTarget for i32 {}
unsafe impl DmaTarget for i64 {}
unsafe impl DmaTarget for isize {}

unsafe impl<T: DmaTarget> DmaTarget for [T] {}
unsafe impl<T: DmaTarget> DmaTarget for MaybeUninit<T> {}

macro_rules! write_target_array_impls {
    ( $( $i:expr, )+ ) => {
        $(
            unsafe impl<T: DmaTarget> DmaTarget for [T; $i] {}
        )+
    };
}

#[rustfmt::skip]
write_target_array_impls!(
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9,
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
    30, 31, 32,
);

// Ideally we would write this blanket impl based on `AsRef`, to not be
// limited to slices. I.e.:
//
//     unsafe impl<B, T> DmaReadBuffer for B
//     where
//         B: Deref + StableDeref,
//         B::Target: AsRef<T>,
//         T: DmaTarget + ?Sized,
//     { ... }
//
// Rust refuses to compile that though ("unconstrained type parameter").

unsafe impl<B, E> DmaReadBuffer for B
where
    B: Deref + StableDeref,
    B::Target: AsSlice<Element = E>,
    E: DmaTarget,
{
    type Target = [E];

    fn dma_read_buffer(&self) -> (*const Self::Target, usize) {
        let target = self.as_slice();
        let ptr = target as *const Self::Target;
        let len = mem::size_of_val(target);
        (ptr, len)
    }
}

unsafe impl<B, T> DmaWriteBuffer for B
where
    B: DerefMut<Target = T> + StableDeref,
    T: DmaTarget + ?Sized,
{
    type Target = T;

    fn dma_write_buffer(&mut self) -> (*mut Self::Target, usize) {
        let target = self.deref_mut();
        let ptr = target as *mut Self::Target;
        let len = mem::size_of_val(target);
        (ptr, len)
    }
}
