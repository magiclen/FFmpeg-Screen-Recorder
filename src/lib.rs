/*!
# FFmpeg Screen Recorder

This program is a gadget which helps you use **FFmpeg** to record your screen on Linux (X11 only). The video record can be saved as a file, or be streamed via RTMP protocol.
*/

pub mod audio;
pub mod cli;
pub mod ffmpeg;
pub mod monitor;
pub mod position;
pub mod resolution;
pub mod session;
pub mod window_info;

pub use position::Position;
pub use resolution::Resolution;
