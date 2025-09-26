#![no_std]
#![no_main]

use pets::Stack;
use pets::Task;

use defmt_semihosting as _;

const SYSTICKS_PER_SCHED_TICK: u32 = 100_000;

static TASK_LIST: [Task; 3] = [
    Task::new(rabbits, &RABBIT_STACK),
    Task::new(hamsters, &HAMSTER_STACK),
    Task::new(cats, &CAT_STACK),
];

static SCHEDULER: pets::Scheduler = pets::Scheduler::new(&TASK_LIST);

#[cortex_m_rt::entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    defmt::info!("Hello!");
    SCHEDULER.start(cp.SYST, SYSTICKS_PER_SCHED_TICK);
}

static RABBIT_STACK: Stack<1024> = Stack::new();

// #[pets::task]

fn rabbits() -> ! {
    loop {
        defmt::info!("Rabbit! (back in 5)");
        pets::delay(5);
    }
}

static HAMSTER_STACK: Stack<1024> = Stack::new();

// #[pets::task]
fn hamsters() -> ! {
    loop {
        defmt::info!("Hamster! (back in 10)");
        pets::delay(10);
    }
}

static CAT_STACK: Stack<1024> = Stack::new();

// #[pets::task]
fn cats() -> ! {
    loop {
        defmt::info!("Cat! (back in 3)");
        pets::delay(3);
    }
}
