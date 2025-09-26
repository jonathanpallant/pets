//! A simple example showing how to use pets
//!
//! It starts three tasks, each of which periodically prints a defmt log and
//! then sleeps.

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

#![no_std]
#![no_main]

use pets::{Scheduler, Stack, Task};

use defmt_semihosting as _;

const SYSTICKS_PER_SCHED_TICK: u32 = 100_000;

static TASK_LIST: [Task; 3] = [
    Task::new(rabbits, &RABBIT_STACK),
    Task::new(hamsters, &HAMSTER_STACK),
    Task::new(cats, &CAT_STACK),
];

static SCHEDULER: Scheduler = Scheduler::new(&TASK_LIST);

#[cortex_m_rt::entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    defmt::info!("Hello!");
    SCHEDULER.start(cp.SYST, SYSTICKS_PER_SCHED_TICK);
}

static RABBIT_STACK: Stack<1024> = Stack::new();

/// Our 'rabbit' task
fn rabbits() -> ! {
    loop {
        defmt::info!("Rabbit! (back in 5)");
        pets::delay(5);
    }
}

static HAMSTER_STACK: Stack<1024> = Stack::new();

/// Our 'hamster' task
fn hamsters() -> ! {
    loop {
        defmt::info!("Hamster! (back in 10)");
        pets::delay(10);
    }
}

static CAT_STACK: Stack<1024> = Stack::new();

/// Our 'cat' task
fn cats() -> ! {
    loop {
        defmt::info!("Cat! (back in 3)");
        pets::delay(3);
    }
}

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
