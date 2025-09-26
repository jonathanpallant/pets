//! Holds the [`Stack`] type and methods

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::UnsafeCell;

/// A task stack, with the given size `LEN` bytes.
///
/// The value of `LEN` must be a multiple of 4, which is checked with an
/// assert.
///
/// We align stacks on 8-byte boundaries, as required by AAPCS.
#[repr(align(8))]
pub struct Stack<const LEN: usize> {
    /// The memory reserved for the task stack
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
        // SAFETY: Pointing one past this object is allowed, as this is full
        // descending stack and we never write to the 'top' address - only
        // below it
        unsafe { self.contents.get().add(1) as *mut u32 }
    }
}

/// SAFETY: Our stack object only exposes pointers to itself, so is thread-safe
/// despite containing an `UnsafeCell`.
unsafe impl<const LEN: usize> Sync for Stack<LEN> {}

impl<const LEN: usize> Default for Stack<LEN> {
    fn default() -> Self {
        Stack::new()
    }
}

// End of File
