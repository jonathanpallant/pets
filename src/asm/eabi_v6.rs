//! Armv6-M EABI code

use crate::{Scheduler, Task, scheduler};

/// PendSV Handler for Armv6-M or Armv8-M Baseline EABI
///
/// This is the task switch code. It is called by hardware when the PendSV bit
/// is set and all other interrupts have finished. It uses only the Armv6-M
/// subset instructions.
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

    // r12 = the handler stack pointer
    mov     r12, sp

    // if current task ID is -1, skip the stacking of the current task
    movs    r0, #1
    cmn     r2, r0
    beq     1f

    //
    // Stack the current task
    //
    // r1 holds the scheduler object's address
    // r2 holds the current task ID
    // r3 holds the task list's address
    //

    // r2 = the current task byte offset 
    lsls    r2, {task_size_bits}

    // sp = the current task stack pointer
    mrs     r0, psp
    mov     sp, r0

    // Push the additional state into stack at sp
    push    {{ lr }}
    push    {{ r4 - r7 }}
    mov     r4, r8
    mov     r5, r9
    mov     r6, r10
    mov     r7, r11
    push    {{ r4 - r7 }}

    // save the adjusted stack pointer to the task object
    mov     r0, sp
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
    lsls    r2, {task_size_bits}

    // sp = the stack pointer from the task object
    ldr     r0, [r3, r2]
    mov     sp, r0

    // Pop the additional state from it
    pop     {{ r4 - r7 }}
    mov     r8, r4
    mov     r9, r5
    mov     r10, r6
    mov     r11, r7
    pop     {{ r4 - r7 }}
    pop     {{ r0 }}
    mov     lr, r0

    // psp = the adjusted task stack pointer
    mov     r0, sp
    msr     psp, r0 

    // restore the handler stack pointer from r12
    mov     sp, r12

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
