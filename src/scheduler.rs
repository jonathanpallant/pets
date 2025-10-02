//! Contains the [`Scheduler`] type

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use crate::{StackPusher, Task};

/// The location of our one and only [`Scheduler`] object.
///
/// We need this so that the free-standing PendSV handler knows where all our system state is.
pub(crate) static SCHEDULER_PTR: AtomicPtr<Scheduler> = AtomicPtr::new(core::ptr::null_mut());

/// Represents a Task
#[derive(Copy, Clone, Debug)]
pub struct TaskId(usize);

impl TaskId {
    /// Represents the Task ID we produce when the scheduler isn't running
    const INVALID_ID: usize = usize::MAX;

    /// Is this the invalid Task ID?
    pub const fn is_invalid(self) -> bool {
        self.0 == Self::INVALID_ID
    }

    /// Create an invalid Task ID
    pub(crate) const fn invalid() -> TaskId {
        TaskId(Self::INVALID_ID)
    }
}

impl defmt::Format for TaskId {
    fn format(&self, fmt: defmt::Formatter) {
        if self.is_invalid() {
            defmt::write!(fmt, "T---");
        } else {
            defmt::write!(fmt, "T{=usize:03}", self.0);
        }
    }
}

impl core::fmt::Display for TaskId {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_invalid() {
            write!(fmt, "T---")
        } else {
            write!(fmt, "T{:03}", self.0)
        }
    }
}

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
    #[cfg(arm_abi = "eabi")]
    pub(crate) const MIN_STACK_SIZE: usize = (4 * 16) + 8;

    /// This is the minimum stack we can support, because of the state we need to push
    ///
    /// Make space for sixteen 32-bit registers, thirty-two 32-bit FPU
    /// registers, plus FPU status register, in the task state, plus some
    /// headroom
    #[cfg(arm_abi = "eabihf")]
    pub(crate) const MIN_STACK_SIZE: usize = (4 * 49) + 8;

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
        syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
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
            // R0-R3
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);

            // Additional task state we persist

            // Extra copy of LR so we can check for FPU status. This copy does
            // not have the FPU bit set, so we don't need to push an Extended
            // Frame above, or the other 16 FPU registers, into the initial
            // state. This will return us to Thread Mode, Process Stack.
            stack_pusher.push(0xFFFFFFFD);

            // R4 - R11
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
            stack_pusher.push(0);
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
        for task in self.task_list.iter() {
            task.unpark();
        }

        #[cfg(not(any(arm_architecture = "v6-m", arm_architecture = "v8-m.base")))]
        self.ticks.fetch_add(1, Ordering::Relaxed);

        #[cfg(any(arm_architecture = "v6-m", arm_architecture = "v8-m.base"))]
        cortex_m::interrupt::free(|_| {
            self.ticks.store(
                self.ticks.load(Ordering::Relaxed).wrapping_add(1),
                Ordering::Relaxed,
            );
        });

        match self.pick_next_task() {
            TaskSelection::NewTask(task_id) => {
                self.next_task.store(task_id.0, Ordering::Relaxed);
                cortex_m::peripheral::SCB::set_pendsv();
            }
            TaskSelection::CurrentTask | TaskSelection::NoTasks => {
                // nothing to
            }
        }
    }

    /// Get current tick count
    pub fn now(&self) -> u32 {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Switch tasks, because this one has nothing to do right now
    pub fn yield_until_tick(&self) {
        let task_id = self.current_task.load(Ordering::Relaxed);
        defmt::trace!("- yield_until_tick on T{=usize:03}", task_id);
        let task = &self.task_list[task_id];
        task.park();
        match self.pick_next_task() {
            TaskSelection::NewTask(task_id) => {
                self.next_task.store(task_id.0, Ordering::Relaxed);
                cortex_m::peripheral::SCB::set_pendsv();
            }
            TaskSelection::CurrentTask => {
                panic!("Picked a task we just parked?!");
            }
            TaskSelection::NoTasks => {
                defmt::trace!("- Sleep!");
                cortex_m::asm::wfi();
                cortex_m::asm::isb();
            }
        }
    }

    /// Get the current Task ID
    pub fn current_task_id(&self) -> TaskId {
        TaskId(self.current_task.load(Ordering::Relaxed))
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
    ///
    /// Returns `true` if a new task was picked, or `false` if no tasks were available
    fn pick_next_task(&self) -> TaskSelection {
        defmt::trace!("> picking a task");
        let task_sel = cortex_m::interrupt::free(|_cs| {
            let current_task = self.current_task.load(Ordering::Relaxed);
            if current_task == usize::MAX {
                return TaskSelection::NewTask(TaskId(0));
            }
            let mut selected_next_task = None;
            let num_tasks = self.task_list.len();
            // Go through all the tasks. We start with the one after the
            // current task, so we don't keep pickng the same task.
            for mut idx in (current_task + 1)..=(current_task + num_tasks) {
                // do the wrap-around
                while idx >= num_tasks {
                    idx -= num_tasks;
                }
                let task = &self.task_list[idx];
                // is this a task we can run right now?
                if !task.parked() {
                    selected_next_task = Some(idx);
                    // no sense in checking any more tasks
                    break;
                }
            }

            if let Some(task_id) = selected_next_task {
                if task_id == current_task {
                    TaskSelection::CurrentTask
                } else {
                    TaskSelection::NewTask(TaskId(task_id))
                }
            } else {
                TaskSelection::NoTasks
            }
        });

        defmt::trace!("< picked {}", task_sel);
        task_sel
    }
}

/// Describes which task we picked
#[derive(defmt::Format)]
enum TaskSelection {
    /// We picked a new task - do a task switch
    NewTask(TaskId),
    /// We like the current task - no switch required
    CurrentTask,
    /// There are no tasks - you should probably sleep
    NoTasks,
}

// End of File
