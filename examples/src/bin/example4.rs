//! An example with three tasks that all run once per tick

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: GPL-3.0-or-later

#![no_std]
#![no_main]

use pets::{Scheduler, Stack, Task};

use pets_examples as _;

const SYSTICKS_PER_SCHED_TICK: u32 = 100_000;

static SCHEDULER: Scheduler = Scheduler::new({
    static TASK_LIST: [Task; 3] = [
        Task::new(rabbits, {
            static STACK: Stack<1024> = Stack::new();
            &STACK
        }),
        Task::new(hamsters, {
            static STACK: Stack<1024> = Stack::new();
            &STACK
        }),
        Task::new(cats, {
            static STACK: Stack<1024> = Stack::new();
            &STACK
        }),
    ];
    &TASK_LIST
});

#[cortex_m_rt::entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    defmt::info!("Hello!");
    SCHEDULER.start(cp.SYST, SYSTICKS_PER_SCHED_TICK);
}

/// Our 'rabbit' task
fn rabbits() -> ! {
    loop {
        defmt::info!("Rabbit!");
        pets::delay(0);
    }
}

/// Our 'hamster' task
fn hamsters() -> ! {
    loop {
        defmt::info!("Hamster!");
        pets::delay(0);
    }
}

/// Our 'cat' task
fn cats() -> ! {
    loop {
        defmt::info!("Cat!");
        pets::delay(0);
    }
}

// End of File
