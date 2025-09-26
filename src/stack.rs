//! Holds the [`Stack`] type and methods

use crate::UnsafeCell;

/// A task stack, with the given size `LEN` bytes.
///
/// The value of `LEN` must be a multiple of 4.
#[repr(align(32))]
pub struct Stack<const LEN: usize> {
    contents: UnsafeCell<[u8; LEN]>,
}

impl<const LEN: usize> Stack<LEN> {
    /// Create a new stack
    pub const fn new() -> Self {
        assert!(LEN.is_multiple_of(4));
        Self {
            contents: UnsafeCell::new([0u8; LEN]),
        }
    }

    /// Get the top of the stack
    pub const fn top(&self) -> *mut u32 {
        unsafe {
            let stack_bottom = self.contents.get() as *mut u8;
            let stack_top = stack_bottom.add(LEN);
            stack_top as *mut u32
        }
    }
}

unsafe impl<const LEN: usize> Sync for Stack<LEN> {}

impl<const LEN: usize> Default for Stack<LEN> {
    fn default() -> Self {
        Stack::new()
    }
}
