//! Common panic/fault/timestamp handlers for the examples

#![no_std]

use defmt_semihosting as _;

/// Called when a panic occurs.
///
/// Logs the panic to defmt and then crashes the CPU.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::println!("PANIC: {}", defmt::Debug2Format(info));
    cortex_m::asm::udf();
}

/// Called when a HardFault occurs.
///
/// Logs the fault to defmt and then crashes the CPU.
#[cortex_m_rt::exception]
unsafe fn HardFault(info: &cortex_m_rt::ExceptionFrame) -> ! {
    defmt::println!("FAULT: {}", defmt::Debug2Format(info));
    cortex_m::asm::udf();
}

// Log scheduler ticks in the defmt logs
defmt::timestamp!("{=u32:010} {}", pets::now(), pets::task_id());

// End of File
