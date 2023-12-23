use std::mem;

/// Trait for unsafely detaching an immutable borrow, attaching a new lifetime to it.
pub trait UnsafelyDetachBorrow<'a, 'b, T> {
    /// Unsafely detaches an immutable borrow, attaching a new lifetime.
    ///
    /// This method primarily allows a borrow to be used alongside other mutable borrows of the
    /// same lifetime. It should not be used lightly and it should likely only be used alongside an
    /// explanation of the safety guarantee.
    unsafe fn unsafely_detach_borrow(&'a self) -> &'b T;
}

impl<'a, 'b, T> UnsafelyDetachBorrow<'a, 'b, T> for &'a T {
    unsafe fn unsafely_detach_borrow(&'a self) -> &'b T {
        mem::transmute(self)
    }
}
