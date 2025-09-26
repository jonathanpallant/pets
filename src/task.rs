//! Holds the [`Task`] type and methods

use core::sync::atomic::{AtomicPtr, Ordering};

use crate::Stack;

pub type TaskEntryFn = fn() -> !;

/// Represents a task that the scheduler is managing
#[repr(C)]
pub struct Task {
    stack: AtomicPtr<u32>,
    entry_fn: TaskEntryFn,
}

impl Task {
    /// Create a new [`Task`] object
    pub const fn new<const N: usize>(entry_fn: TaskEntryFn, stack: &Stack<N>) -> Task {
        Task {
            entry_fn,
            stack: AtomicPtr::new(stack.top()),
        }
    }

    /// Get the initial entry function for this task
    pub const fn entry_fn(&self) -> TaskEntryFn {
        self.entry_fn
    }

    /// Get the current stack pointer for this task
    pub fn stack(&self) -> *mut u32 {
        self.stack.load(Ordering::Relaxed)
    }

    /// Set the current stack pointer for this task
    ///
    /// # Safety
    ///
    /// The task will execute using the stack given, so it must point to the
    /// last item in a valid Arm EABI stack, with a full pets Stack Frame
    /// proceeding it.
    pub unsafe fn set_stack(&self, new_stack: *mut u32) {
        self.stack.store(new_stack, Ordering::Relaxed)
    }
}

unsafe impl Sync for Task {}
