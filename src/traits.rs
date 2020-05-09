//! `unsafe` traits for buffers usable with DMA.
//!
//! The traits defined here are concerned with ensuring requirements
//! 1, 2, and 4 from the README. They ignore requirement 3 (`'static` bound)
//! to make them useful for DMA on stack buffers too. Requirement 3 must be
//! enforced by the `Transfer` implementation instead.

use core::ops::{Deref, DerefMut};
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
    type Word;

    /// Provide a buffer usable for DMA reads.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in words
    ///
    /// # Safety
    ///
    /// - This function must always return the same values, if called multiple
    ///   times.
    /// - The memory specified by the returned pointer and size must not be
    ///   freed as long as `self` is not dropped.
    fn dma_read_buffer(&self) -> (*const Self::Word, usize);
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
    type Word;

    /// Provide a buffer usable for DMA writes.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in words
    ///
    /// # Safety
    ///
    /// - This function must always return the same values, if called multiple
    ///   times.
    /// - The memory specified by the returned pointer and size must not be
    ///   freed as long as `self` is not dropped.
    fn dma_write_buffer(&mut self) -> (*mut Self::Word, usize);
}

// Blanked implementations for common DMA buffer types.

unsafe impl<B, T> DmaReadBuffer for B
where
    B: Deref<Target = T> + StableDeref,
    T: private::DmaReadTarget + ?Sized,
{
    type Word = T::Word;

    fn dma_read_buffer(&self) -> (*const Self::Word, usize) {
        self.as_dma_read_buffer()
    }
}

unsafe impl<B, T> DmaWriteBuffer for B
where
    B: DerefMut<Target = T> + StableDeref,
    T: private::DmaWriteTarget + ?Sized,
{
    type Word = T::Word;

    fn dma_write_buffer(&mut self) -> (*mut Self::Word, usize) {
        self.as_dma_write_buffer()
    }
}

/// This module contains traits and impls used by the blanket impls of
/// the DMA buffer traits.
///
/// It is kept private to prevent others from implementing these traits.
/// Third-party code should impl the public DMA buffer traits directly.
mod private {
    use core::mem::{self, MaybeUninit};

    /// Trait for DMA word types used by the blanket implementations.
    ///
    /// # Safety
    ///
    /// Types that implement this trait must be valid for every possible byte
    /// pattern. This is to ensure that, whatever DMA writes into the buffer,
    /// we won't get UB due to invalid values.
    pub unsafe trait DmaWord {}

    unsafe impl DmaWord for u8 {}
    unsafe impl DmaWord for u16 {}
    unsafe impl DmaWord for u32 {}

    /// Trait for `Deref` targets used by the blanket `DmaReadBuffer` impl.
    ///
    /// This trait exists solely to work around
    /// https://github.com/rust-lang/rust/issues/20400.
    ///
    /// # Safety
    ///
    /// - `as_dma_read_buffer` must adhere to the safety requirements
    ///   documented for `DmaReadBuffer::dma_read_buffer`.
    pub unsafe trait DmaReadTarget {
        type Word: DmaWord;

        fn as_dma_read_buffer(&self) -> (*const Self::Word, usize) {
            let ptr = self as *const _ as *const Self::Word;
            let len = mem::size_of_val(self) / mem::size_of::<Self::Word>();
            (ptr, len)
        }
    }

    /// Trait for `DerefMut` targets used by the blanket `DmaWriteBuffer` impl.
    ///
    /// This trait exists solely to work around
    /// https://github.com/rust-lang/rust/issues/20400.
    ///
    /// # Safety
    ///
    /// - `as_dma_write_buffer` must adhere to the safety requirements
    ///   documented for `DmaWriteBuffer::dma_write_buffer`.
    pub unsafe trait DmaWriteTarget {
        type Word: DmaWord;

        fn as_dma_write_buffer(&mut self) -> (*mut Self::Word, usize) {
            let ptr = self as *mut _ as *mut Self::Word;
            let len = mem::size_of_val(self) / mem::size_of::<Self::Word>();
            (ptr, len)
        }
    }

    // Support DMA reads and writes on the Word types themselves.

    unsafe impl<W: DmaWord> DmaReadTarget for W {
        type Word = W;
    }

    unsafe impl<W: DmaWord> DmaWriteTarget for W {
        type Word = W;
    }

    // Support DMA reads and writes on slices.

    unsafe impl<T: DmaReadTarget> DmaReadTarget for [T] {
        type Word = T::Word;
    }

    unsafe impl<T: DmaWriteTarget> DmaWriteTarget for [T] {
        type Word = T::Word;
    }

    // Support DMA reads and writes on arrays.

    macro_rules! dma_target_array_impls {
        ( $( $i:expr, )+ ) => {
            $(
                unsafe impl<T: DmaReadTarget> DmaReadTarget for [T; $i] {
                    type Word = T::Word;
                }

                unsafe impl<T: DmaWriteTarget> DmaWriteTarget for [T; $i] {
                    type Word = T::Word;
                }
            )+
        };
    }

    #[rustfmt::skip]
    dma_target_array_impls!(
        0,  1,  2,  3,  4,  5,  6,  7,  8,  9,
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32,
    );

    // Support DMA writes on MaybeUninit data.

    unsafe impl<T: DmaWriteTarget> DmaWriteTarget for MaybeUninit<T> {
        type Word = T::Word;
    }
}
