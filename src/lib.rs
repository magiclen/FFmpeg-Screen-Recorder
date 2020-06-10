//! # FFmpeg Screen Recorder
//!
//! This program is a gadget which helps you use FFmpeg to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol.

extern crate num_cpus;

#[macro_use]
extern crate execute;

mod position;
mod resolution;
mod window_info;

use std::fs;

pub use position::*;
pub use resolution::*;
pub use window_info::*;

#[inline]
pub fn try_delete_file(file_path: &str) {
    if fs::remove_file(file_path).is_err() {}
}

#[inline]
pub fn get_number_of_processors() -> usize {
    num_cpus::get()
}
