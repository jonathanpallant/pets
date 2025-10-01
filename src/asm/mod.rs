//! Appropriate assembly language routines for the architecture

#[cfg(all(
    arm_abi = "eabi",
    any(arm_architecture = "v6-m", arm_architecture = "v8-m.base")
))]
mod eabi_v6;

#[cfg(all(
    arm_abi = "eabi",
    not(any(arm_architecture = "v6-m", arm_architecture = "v8-m.base"))
))]
mod eabi;

#[cfg(arm_abi = "eabihf")]
mod eabihf;
