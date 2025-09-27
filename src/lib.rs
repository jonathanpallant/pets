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

use core::{arch::naked_asm, cell::UnsafeCell};

pub use scheduler::Scheduler;
pub use stack::Stack;
pub use task::Task;

use scheduler::TaskId;
use stack_pusher::StackPusher;

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

/// PendSV Handler
///
/// This is the task switch code. It is called by hardware when the PendSV bit
/// is set and all other interrupts have finished.
///
/// On entry, we will find that PC, LR, R12, R3, R2, R1 and R0 will have been
/// pushed onto the PSP. We thus push the remaining registers (which are as
/// the running task left them) and then restore the registers from another
/// task. Exiting from this function will cause the hardware to restore PC,
/// LR, R12, R3, R2, R1, and R0 from the new tasks PSP, and so the new task
/// will resume.
///
/// It is a naked function because we do not want the compiler pushing
/// anything else to the stack and re-using registers containing precious task
/// state.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn PendSV() {
    // NOTE: This code must NOT touch r4-r11. It can ONLY touch r0-r3 and r12,
    // because those registers were stacked by the hardare on exception entry.

    naked_asm!(r#"
    // r1 = the address of the Scheduler object
    ldr     r1, ={scheduler_ptr}
    ldr     r1, [r1]

    // r2 = the current task ID
    ldr     r2, [r1, {current_task_offset}]

    // r3 = the task list pointer
    ldr     r3, [r1, {task_list_offset}]

    // if current task ID is -1, skip the stacking of the current task
    cmp     r2, #-1
    beq     1f

    //
    // Stack the current task
    //
    // r1 holds the scheduler object's address
    // r2 holds the current task ID
    // r3 holds the task list's address
    //

    // r2 = the current task byte offset 
    lsl     r2, {task_size_bits}

    // r0 = the current task stack pointer
    mrs     r0, psp

    // Push the additional state into stack at r0
    stmdb   r0!, {{ r4 - r11 }}

    // save the stack pointer (in r0) to the task object
    str     r0, [r3, r2]

    //
    // Pop the next task
    //
    // r1 holds the scheduler object's address
    // r3 holds the task list's address
    //

    1:

    // r2 = the next task byte offset
    ldr     r2, [r1, {next_task_offset}]
    lsl     r2, {task_size_bits}

    // r0 = the stack pointer from the task object
    ldr     r0, [r3, r2]

    // Pop the additional state from it
    ldmia   r0!, {{ r4 - r11 }}

    // Set the current task stack pointer
    msr     psp, r0

    //
    // Update the Current Task ID
    //
    // r1 holds the scheduler object's address
    //

    // copy the next task id to the current task id
    ldr     r2, [r1, {next_task_offset}]
    str     r2, [r1, {current_task_offset}]

    //
    // return to thread mode on the process stack
    //

    // This is the magic LR value for 'return to thread mode process stack'
    mov     lr, #0xFFFFFFFD
    bx      lr
    "#,
    scheduler_ptr = sym scheduler::SCHEDULER_PTR,
    current_task_offset = const Scheduler::CURRENT_TASK_OFFSET,
    next_task_offset = const Scheduler::NEXT_TASK_OFFSET,
    task_list_offset = const Scheduler::TASK_LIST_OFFSET,
    task_size_bits = const Task::SIZE_BITS,
    );
}

// End of File
