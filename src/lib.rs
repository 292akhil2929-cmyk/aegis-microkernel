#![no_std]

pub mod capability;
pub mod ipc;
pub mod memory;
pub mod paging;
pub mod scheduler;
pub mod syscall;

#[cfg(test)]
extern crate std;
