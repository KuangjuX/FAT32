#![no_std]

extern crate alloc;

pub mod device;
pub mod file;
pub mod superblock;
pub mod bpb;
pub mod fat;
pub mod utils;
pub mod entry;

pub const BUFFER_SIZE:usize = 512;