#![no_std]

use core::{
    arch::naked_asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicPtr, Ordering},
};

mod scheduler;
mod stack;
mod stack_pusher;
mod task;

pub use scheduler::Scheduler;
pub use stack::Stack;
use stack_pusher::StackPusher;
pub use task::Task;

static SCHEDULER_PTR: AtomicPtr<Scheduler> = AtomicPtr::new(core::ptr::null_mut());

/// Delay a task for at least the given period
pub fn delay(ticks: u32) {
    let scheduler_ptr = SCHEDULER_PTR.load(Ordering::Relaxed);
    let scheduler = unsafe { &*scheduler_ptr };
    let start = scheduler.now();
    loop {
        let delta = scheduler.now().wrapping_sub(start);
        if delta > ticks {
            break;
        }
    }
}

/// Get the current time in ticks
pub fn now() -> u32 {
    let scheduler_ptr = SCHEDULER_PTR.load(Ordering::Relaxed);
    if scheduler_ptr.is_null() {
        0xFFFFFFFF
    } else {
        let scheduler = unsafe { &*scheduler_ptr };
        scheduler.now()
    }
}

/// SysTick Handler
#[unsafe(no_mangle)]
extern "C" fn SysTick() {
    let scheduler_ptr = SCHEDULER_PTR.load(Ordering::Relaxed);
    let scheduler = unsafe { &*scheduler_ptr };
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
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn PendSV() {
    naked_asm!(r#"
	// r1 = the address of the Scheduler object
	ldr     r1, ={scheduler_ptr}
	ldr     r1, [r1]

	// r2 = the current task ID - it's the first word in the Scheduler struct
	ldr     r2, [r1, 0]

	// r3 = the task list pointer - it's the third word in the Scheduler struct
	ldr     r3, [r1, 8]

	// if current task ID is -1, skip the stacking
	cmp     r2, #-1
	beq     1f

	// r2 = the current task byte offset 
	lsl     r2, 3

	// r0 = the current task stack pointer
	mrs     r0, psp

	// Push the additional state into stack at r0
	stmfd   r0!, {{ r4 - r11 }}

	// save the stack pointer (in r0) to the task object
	str     r0, [r3, r2]

	1:

	// r2 = the next task byte offset - it's the second word in the Scheduler struct
	ldr     r2, [r1, 4]
	lsl     r2, 3

	// r0 = the stack pointer from the task object
	ldr     r0, [r3, r2]

	// Pop the additional state from it
	ldmfd   r0!, {{ r4 - r11 }}

	// Set the current task stack pointer
	msr     psp, r0

	// copy the next task id to the current task id
	ldr     r2, [r1, 4]
	str     r2, [r1, 0]

	// return to thread mode on the process stack
	mov     lr, #0xFFFFFFFD
	bx      lr
	"#,
	scheduler_ptr = sym SCHEDULER_PTR);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::println!("PANIC: {}", defmt::Debug2Format(info));
    cortex_m::asm::udf();
}

#[cortex_m_rt::exception]
unsafe fn HardFault(info: &cortex_m_rt::ExceptionFrame) -> ! {
    defmt::println!("FAULT: {}", defmt::Debug2Format(info));
    cortex_m::asm::udf();
}

defmt::timestamp!("{=u32:010}", now());
