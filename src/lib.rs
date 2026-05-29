#![cfg_attr(target_arch = "bpf", no_std)]

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

pub use processor::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
