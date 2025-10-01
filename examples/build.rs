//! Build Script for the PETS examples
//!
//! Not required when using PETS as a library

// Copyright (c) 2025 Ferrous Systems
// SPDX-License-Identifier: CC0-1.0

use std::{env, error::Error, fs, path::PathBuf};

use arm_targets::Arch;

fn main() -> Result<(), Box<dyn Error>> {
    let target_info = arm_targets::process();
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    // put memory layout (linker script) in the linker search path as the
    // package root isn't always searched

    let memory_file = match target_info.arch() {
        Some(Arch::Armv6M | Arch::Armv7M | Arch::Armv7EM) => "memory_mps2.x",
        Some(Arch::Armv8MBase | Arch::Armv8MMain) => "memory_mps2tz.x",
        _ => {
            panic!("Target {:?} not supported", std::env::var("TARGET"));
        }
    };
    fs::copy(memory_file, out_dir.join("memory.x"))?;
    // important - if the file changes, re-run the build
    println!("cargo::rerun-if-changed={}", memory_file);
    // tell the linker where to find them
    println!("cargo::rustc-link-search={}", out_dir.display());
    Ok(())
}

// End of File
