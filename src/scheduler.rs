//! Contains the [`Scheduler`] type

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::{StackPusher, Task};

/// A pre-emptive task-switching scheduler
///
/// It time slices tasks in a round-robin fashion, whether or not they have work to do.
///
/// The Arm hardware will push {CPSR, PC, LR, R12, R3, R2, R1, R0} to PSP when an
/// exception occurs. We then push the rest (R11 to R4).
#[repr(C)]
pub struct Scheduler {
    /// Which task is currently running
    ///
    /// The asm relies on this being the first field in this struct
    current_task: AtomicUsize,
    /// Which task should PendSV switch to next
    ///
    /// The asm relies on this being the second field in this struct
    next_task: AtomicUsize,
    /// A list of tasks
    tasks: &'static [Task],
    /// Current tick count
    ticks: AtomicU32,
}

impl Scheduler {
    const DEFAULT_CPSR: u32 = 1 << 24;

    /// Build the scheduler
    pub const fn new(tasks: &'static [Task]) -> Scheduler {
        Scheduler {
            tasks,
            current_task: AtomicUsize::new(usize::MAX),
            next_task: AtomicUsize::new(0),
            ticks: AtomicU32::new(0),
        }
    }

    /// Run the scheduler
    ///
    /// You may only call this once, and you should call it from `fn main()`
    /// once all your hardware is configured. We should be in Handler mode on
    /// the Main stack.
    pub fn start(&self, mut syst: cortex_m::peripheral::SYST, systicks_per_sched_tick: u32) -> ! {
        if self.current_task.load(Ordering::SeqCst) != usize::MAX {
            panic!("Tried to re-start scheduler!");
        }

        // remember where this object is - it cannot move because we do not exit this function
        let self_addr = self as *const Scheduler as *mut Scheduler;
        defmt::info!("Scheduler @ {=usize:08x}", self_addr as usize);
        defmt::info!(
            "SCHEDULER_PTR @ {=usize:08x}",
            core::ptr::addr_of!(crate::SCHEDULER_PTR) as usize
        );
        crate::SCHEDULER_PTR.store(self_addr, Ordering::Relaxed);

        // Must do this /after/ setting SCHEDULER_PTR
        syst.set_reload(systicks_per_sched_tick);
        syst.clear_current();
        syst.enable_counter();
        syst.enable_interrupt();

        // We need to push some empty state into each task stack
        for (task_idx, task) in self.tasks.iter().enumerate() {
            let old_stack_top = task.stack();
            defmt::info!(
                "Init task frame {=usize}, with stack @ 0x{=usize:08x}",
                task_idx,
                old_stack_top as usize
            );
            let mut stack_pusher = unsafe { StackPusher::new(old_stack_top) };

            // Standard Arm exception frame

            // CPSR
            stack_pusher.push(Self::DEFAULT_CPSR);
            // PC
            stack_pusher.push(task.entry_fn() as usize as u32);
            // LR
            stack_pusher.push(0xFF00000D);
            // R12
            stack_pusher.push(0xFF00000C);
            // R3
            stack_pusher.push(0xFF000003);
            // R2
            stack_pusher.push(0xFF000002);
            // R1
            stack_pusher.push(0xFF000001);
            // R0
            stack_pusher.push(0xFF000000);

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

            // Set task stack pointer to the last thing we pushed

            defmt::info!(
                "Fini task frame {=usize}, with stack @ 0x{=usize:08x}",
                task_idx,
                stack_pusher.current() as usize
            );
            unsafe {
                task.set_stack(stack_pusher.current());
            }
        }

        // Fire the PendSV exception - the PendSV handler will select a task
        // to run and run it
        defmt::debug!("Hit PendSV");
        cortex_m::peripheral::SCB::set_pendsv();

        // PendSV can take a few clock cycles to fire
        #[allow(clippy::empty_loop)]
        loop {}
    }

    /// Call periodically, to get the scheduler to adjust which task should run next
    ///
    /// This is currently a round-robin with no priorities, and no sense of tasks being blocked
    ///
    /// Ideally call this from a SysTick handler
    pub fn sched_tick(&self) {
        defmt::debug!("Tick!");
        self.ticks.fetch_add(1, Ordering::Relaxed);
        let next_task = self.next_task.load(Ordering::Relaxed);
        let maybe_next_task = next_task + 1;
        let new_next_task = if maybe_next_task >= self.tasks.len() {
            0
        } else {
            maybe_next_task
        };
        self.next_task.store(new_next_task, Ordering::Relaxed);
        cortex_m::peripheral::SCB::set_pendsv();
    }

    /// Get current tick count
    pub fn now(&self) -> u32 {
        self.ticks.load(Ordering::Relaxed)
    }
}
