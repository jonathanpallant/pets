//! Contains the [`Scheduler`] type

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use crate::{StackPusher, Task};

/// The location of our one and only [`Scheduler`] object.
///
/// We need this so that the free-standing PendSV handler knows where all our system state is.
pub(crate) static SCHEDULER_PTR: AtomicPtr<Scheduler> = AtomicPtr::new(core::ptr::null_mut());

/// A pre-emptive task-switching scheduler
///
/// It time slices tasks in a round-robin fashion, whether or not they have work to do.
///
/// The Arm hardware will push {CPSR, PC, LR, R12, R3, R2, R1, R0} to PSP when an
/// exception occurs. We then push the rest (R11 to R4).
#[repr(C)]
pub struct Scheduler {
    /// Which task is currently running
    current_task: AtomicUsize,
    /// Which task should PendSV switch to next
    next_task: AtomicUsize,
    /// A fixed, static list of all our tasks
    task_list: &'static [Task],
    /// Current tick count
    ticks: AtomicU32,
}

impl Scheduler {
    /// The offset, in bytes, to the `current_task` field
    pub(crate) const CURRENT_TASK_OFFSET: usize = core::mem::offset_of!(Scheduler, current_task);

    /// The offset, in bytes, to the `next_task` field
    pub(crate) const NEXT_TASK_OFFSET: usize = core::mem::offset_of!(Scheduler, next_task);

    /// The offset, in bytes, to the `tasks` field
    pub(crate) const TASK_LIST_OFFSET: usize = core::mem::offset_of!(Scheduler, task_list);

    /// This is the minimum stack we can support, because of the state we need to push
    ///
    /// Make space for sixteen 32-bit registers in the task state, plus some
    /// headroom
    pub(crate) const MIN_STACK_SIZE: usize = (4 * 16) + 8;

    /// The value of the Processor Status Register when a task starts
    ///
    /// The only bit we need to set is the T bit, to indicate that the
    /// task should run in Thumb mode (the only supported mode on Armv7-M)
    const DEFAULT_CPSR: u32 = 1 << 24;

    /// Build the scheduler
    pub const fn new(task_list: &'static [Task]) -> Scheduler {
        // Cannot schedule without at least one task
        assert!(!task_list.is_empty());
        Scheduler {
            task_list,
            current_task: AtomicUsize::new(usize::MAX),
            next_task: AtomicUsize::new(0),
            ticks: AtomicU32::new(0),
        }
    }

    /// Run the scheduler
    ///
    /// You may only call this once, and you should call it from `fn main()`
    /// once all your hardware is configured. We should be in Privileged
    /// Thread mode on the Main stack.
    pub fn start(&self, mut syst: cortex_m::peripheral::SYST, systicks_per_sched_tick: u32) -> ! {
        if self.current_task.load(Ordering::SeqCst) != usize::MAX {
            panic!("Tried to re-start scheduler!");
        }

        // remember where this object is - it cannot move because we do not exit this function
        defmt::info!(
            "SCHEDULER_PTR @ {=usize:08x}",
            core::ptr::addr_of!(SCHEDULER_PTR) as usize
        );
        let self_addr = self as *const Scheduler as *mut Scheduler;
        defmt::info!("Scheduler @ {=usize:08x}", self_addr as usize);
        SCHEDULER_PTR.store(self_addr, Ordering::Release);

        // Must do this /after/ setting SCHEDULER_PTR because the SysTick
        // exception handler will use SCHEDULER_PTR
        syst.set_reload(systicks_per_sched_tick);
        syst.clear_current();
        syst.enable_counter();
        syst.enable_interrupt();

        // We need to push some empty state into each task stack
        for (task_idx, task) in self.task_list.iter().enumerate() {
            let old_stack_top = task.stack();
            defmt::info!(
                "Init task frame {=usize}, with stack @ 0x{=usize:08x}",
                task_idx,
                old_stack_top as usize
            );

            // SAFETY: The task constructor does not let us make tasks with
            // stacks that are too small.
            let mut stack_pusher = unsafe { StackPusher::new(old_stack_top) };

            // Standard Arm exception frame

            // CPSR
            stack_pusher.push(Self::DEFAULT_CPSR);
            // PC
            stack_pusher.push(task.entry_fn() as usize as u32);
            // LR
            stack_pusher.push(0);
            // R12
            stack_pusher.push(0);
            // R3
            stack_pusher.push(0);
            // R2
            stack_pusher.push(0);
            // R1
            stack_pusher.push(0);
            // R0
            stack_pusher.push(0);

            // Additional task state we persist

            // R11
            stack_pusher.push(0);
            // R10
            stack_pusher.push(0);
            // R9
            stack_pusher.push(0);
            // R8
            stack_pusher.push(0);
            // R7
            stack_pusher.push(0);
            // R6
            stack_pusher.push(0);
            // R5
            stack_pusher.push(0);
            // R4
            stack_pusher.push(0);

            // Report how much space we used

            defmt::debug!(
                "Fini task frame {=usize}, with stack @ 0x{=usize:08x}",
                task_idx,
                stack_pusher.current() as usize
            );

            // Set task stack pointer to the last thing we pushed

            // SAFETY: the pointer we are passing is a validly aligned stack pointer
            unsafe {
                task.set_stack(stack_pusher.current());
            }
        }

        // Fire the PendSV exception - the PendSV handler will select a task
        // to run and run it
        defmt::debug!("Hit PendSV");
        cortex_m::peripheral::SCB::set_pendsv();
        // flush the pipeline to ensure the PendSV fires before we reach the end of this function
        cortex_m::asm::isb();
        // impossible to get here
        unreachable!();
    }

    /// Call periodically, to get the scheduler to adjust which task should run next
    ///
    /// This is currently a round-robin with no priorities, and no sense of tasks being blocked
    ///
    /// Ideally call this from a SysTick handler
    pub fn sched_tick(&self) {
        defmt::debug!("Tick!");
        self.ticks.fetch_add(1, Ordering::Relaxed);
        self.pick_next_task();
        cortex_m::peripheral::SCB::set_pendsv();
    }

    /// Get current tick count
    pub fn now(&self) -> u32 {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Switch tasks, because this one has nothing to do right now
    pub fn yield_current_task(&self) {
        self.pick_next_task();
        cortex_m::peripheral::SCB::set_pendsv();
    }

    /// Get the handler to the global scheduler
    pub(crate) fn get_scheduler() -> Option<&'static Scheduler> {
        // Get our stashed pointer
        let scheduler_ptr = SCHEDULER_PTR.load(Ordering::Relaxed);
        // Are we intialised?
        if scheduler_ptr.is_null() {
            None
        } else {
            // SAFETY: Only [`Scheduler::start`] writes to [`SCHEDULER_PTR`] and it
            // always sets it to be a valid pointer to a [`Scheduler`] that does not
            // move.
            Some(unsafe { &*scheduler_ptr })
        }
    }

    /// Select the next task in the round-robin
    ///
    /// Updates `self.next_task` but doesn't trigger a task switch. Set PendSV
    /// to do that.
    fn pick_next_task(&self) {
        cortex_m::interrupt::free(|_cs| {
            let next_task = self.next_task.load(Ordering::Relaxed);
            let maybe_next_task = next_task + 1;
            let new_next_task = if maybe_next_task >= self.task_list.len() {
                0
            } else {
                maybe_next_task
            };
            self.next_task.store(new_next_task, Ordering::Relaxed);
        });
    }
}

// End of File
