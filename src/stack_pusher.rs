//! Holds the [`StackPusher`] type and methods

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

/// A helper for pushing things into a full-descending Arm EABI stack
pub(crate) struct StackPusher(*mut u32);

impl StackPusher {
    /// Make a new full-descending stack from the given pointer
    ///
    /// It will not write to the given pointer, but it will write immediately
    /// below it - becuase this is a Full Descending stack.
    ///
    /// # Safety
    ///
    /// There must be enough free space below the given pointer to accept all
    /// the items you are going to push.
    pub(crate) unsafe fn new(stack_top: *mut u32) -> StackPusher {
        StackPusher(stack_top)
    }

    /// Push something onto the stack, incrementing the value
    pub(crate) fn push(&mut self, value: u32) {
        // SAFETY: We required that the stack was large enough when this
        // object was created, so we can assume it is large enough here.
        unsafe {
            self.0 = self.0.offset(-1);
            self.0.write_volatile(value);
        }
    }

    /// Get the current stack value
    pub(crate) fn current(&self) -> *mut u32 {
        self.0
    }
}

// End of File
