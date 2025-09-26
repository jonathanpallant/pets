//! Holds the [`Task`] type and methods

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

use core::sync::atomic::{AtomicPtr, Ordering};

use crate::Stack;

/// The function signature for our task entry functions.
///
/// Our tasks run forever and take no arguments.
pub type TaskEntryFn = fn() -> !;

/// Represents a task that the scheduler is managing
///
/// The size of this struct must be a power of 2 in order for the pendsv
/// assembly code to be able to quickly pick a task out of the list based
/// on an index.
#[repr(C)]
pub struct Task {
    /// The stack pointer for our task
    ///
    /// This is the value taken from PSP when a task is suspended, and is
    /// therefore the value to put back into PSP when the task is resumed.
    ///
    /// When a task is suspended, the 32 bytes after this pointer should be
    /// the stacked task state.
    stack: AtomicPtr<u32>,
    /// The function to call when the task first starts
    entry_fn: TaskEntryFn,
}

impl Task {
    /// The size of a task object is `pow(2, SIZE_BITS)`.
    pub const SIZE_BITS: usize = 3;

    /// A compile-time check that the size of a [`Task`] is what we said it was.
    const _CHECK: () = const {
        assert!(core::mem::size_of::<Self>() == (1 << Self::SIZE_BITS));
    };

    /// Create a new [`Task`] object
    pub const fn new<const N: usize>(entry_fn: TaskEntryFn, stack: &Stack<N>) -> Task {
        assert!(N > crate::Scheduler::MIN_STACK_SIZE);
        Task {
            entry_fn,
            stack: AtomicPtr::new(stack.top()),
        }
    }

    /// Get the initial entry function for this task
    pub(crate) const fn entry_fn(&self) -> TaskEntryFn {
        self.entry_fn
    }

    /// Get the current stack pointer for this task
    pub(crate) fn stack(&self) -> *mut u32 {
        self.stack.load(Ordering::Relaxed)
    }

    /// Set the current stack pointer for this task
    ///
    /// # Safety
    ///
    /// The task will execute using the stack given, so it must point to the
    /// last item in a valid Arm EABI stack, with a full pets Stack Frame
    /// proceeding it.
    pub(crate) unsafe fn set_stack(&self, new_stack: *mut u32) {
        self.stack.store(new_stack, Ordering::Relaxed)
    }
}

// End of File
