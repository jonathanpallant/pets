//! # P.E.T.S - a pre-emptive time slicer
//!
//! PETS is a very basic round-robin pre-emptive scheduler. You can register
//! multiple tasks to execute and it will execute each of them in turn.
//!
//! It currently only works on Arm Cortex-M - either Armv7-M, Armv7E-M or
//! Armv8-M Main should be fine.
//!
//! It's basically an exercise in seeing just how small an RTOS kernel you
//! could get away with, whilst still being somewhat useful.
//!
//! * Copyright (C) 2025 Ferrous Systems
//! * SPDX-License-Identifier: GPL-3.0-or-later

#![no_std]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::missing_docs_in_private_items)]
#![deny(clippy::missing_safety_doc)]

mod scheduler;
mod stack;
mod stack_pusher;
mod task;

use core::cell::UnsafeCell;

pub use scheduler::Scheduler;
pub use stack::Stack;
pub use task::Task;

use scheduler::TaskId;
use stack_pusher::StackPusher;

mod asm;

/// Delay a task for at least the given period, measured in timer ticks.
///
/// Calling `delay(0)` is basically just a yield.
pub fn delay(ticks: u32) {
    defmt::trace!("Sleeping for {} ticks", ticks);
    let scheduler = Scheduler::get_scheduler().unwrap();
    let start = scheduler.now();
    loop {
        // yield first, so delay(0) does at least one task switch
        scheduler.yield_until_tick();
        // is it time to leave?
        let delta = scheduler.now().wrapping_sub(start);
        if delta >= ticks {
            break;
        }
        defmt::trace!("Task {} still sleeping...", task_id());
    }
}

/// Get the current time, in ticks
pub fn now() -> u32 {
    if let Some(scheduler) = Scheduler::get_scheduler() {
        scheduler.now()
    } else {
        0
    }
}

/// Get the currently running task ID
pub fn task_id() -> TaskId {
    if let Some(scheduler) = Scheduler::get_scheduler() {
        scheduler.current_task_id()
    } else {
        TaskId::invalid()
    }
}

/// Our SysTick Handler
///
/// Tells the global scheduler that maybe its time to think about changing
/// which task is running.
#[unsafe(no_mangle)]
extern "C" fn SysTick() {
    let scheduler = Scheduler::get_scheduler().unwrap();
    scheduler.sched_tick();
}

// End of File
