//! Armv7-M EABI code

use crate::{Scheduler, Task, scheduler};

/// PendSV Handler for Armv7-M or Armv8-M Mainline EABI
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

    core::arch::naked_asm!(r#"
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
    stmdb   r0!, {{ r4 - r11, lr }}

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
    ldmia   r0!, {{ r4 - r11, lr }}

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
    // return to the task
    //

    bx       lr
    "#,
    scheduler_ptr = sym scheduler::SCHEDULER_PTR,
    current_task_offset = const Scheduler::CURRENT_TASK_OFFSET,
    next_task_offset = const Scheduler::NEXT_TASK_OFFSET,
    task_list_offset = const Scheduler::TASK_LIST_OFFSET,
    task_size_bits = const Task::SIZE_BITS,
    );
}
