//! Out-of-memory reporting for Cranelift's fallible collections.
//!
//! Cranelift's heap-allocating containers expose `try_*` constructors and
//! growth methods that report allocation failure to the caller instead of
//! aborting the process. Those methods report failure with the [`OutOfMemory`]
//! error defined here, and [`PanicOnOom`] recovers the panicking behaviour at
//! the call sites that leave allocation failure to it.

use core::fmt;

/// An out-of-memory (OOM) error.
///
/// This error is the sentinel for allocation failure due to memory exhaustion.
/// It records the size, in bytes, of the allocation that was attempted and
/// which failed.
///
/// Constructing and inspecting an `OutOfMemory` performs zero allocations.
///
/// # Example
///
/// ```
/// use cranelift_bitset::oom::OutOfMemory;
///
/// let oom = OutOfMemory::new(8192);
/// assert_eq!(oom.requested_allocation_size(), 8192);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutOfMemory {
    requested_allocation_size: usize,
}

impl OutOfMemory {
    /// Construct a new `OutOfMemory` error.
    ///
    /// The `requested_allocation_size` argument should be the size (in bytes)
    /// of the associated allocation that was attempted and failed.
    ///
    /// This operation performs zero allocations.
    #[inline]
    pub const fn new(requested_allocation_size: usize) -> Self {
        Self {
            requested_allocation_size,
        }
    }

    /// Get the size (in bytes) of the associated allocation that was attempted
    /// and which failed.
    #[inline]
    pub fn requested_allocation_size(&self) -> usize {
        self.requested_allocation_size
    }
}

impl fmt::Debug for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutOfMemory")
            .field(
                "requested_allocation_size",
                &self.requested_allocation_size(),
            )
            .finish()
    }
}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "out of memory (failed to allocate {} bytes)",
            self.requested_allocation_size()
        )
    }
}

impl core::error::Error for OutOfMemory {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}

/// An extension trait for ignoring [`OutOfMemory`] errors.
///
/// Use this to unwrap a `Result<T, OutOfMemory>` into its inner `T` or
/// otherwise panic, leveraging the type system to be sure that every error
/// unwrapped this way is an `OutOfMemory`.
pub trait PanicOnOom {
    /// The non-`OutOfMemory` result of calling `panic_on_oom`.
    type Result;

    /// Panic on `OutOfMemory` errors, returning the non-`OutOfMemory` result.
    fn panic_on_oom(self) -> Self::Result;
}

impl<T> PanicOnOom for Result<T, OutOfMemory> {
    type Result = T;

    #[track_caller]
    fn panic_on_oom(self) -> Self::Result {
        match self {
            Ok(x) => x,
            Err(oom) => panic!("unhandled out-of-memory error: {oom}"),
        }
    }
}
