//! # FFmpeg Screen Recorder
//!
//! This program is a gadget which helps you use FFmpeg to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol.

extern crate chrono;
extern crate clap;
extern crate num_cpus;
extern crate subprocess;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use subprocess::Exec;

use chrono::prelude::*;

use clap::{App, Arg};

// TODO -----Config START-----

const APP_NAME: &str = "FFmpeg Screen Recorder";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DEFAULT_FFMPEG_PATH: &str = "ffmpeg";

pub struct Config {
    pub opt_window: bool,
    pub opt_no_sound: bool,
    pub opt_file_path: String,
    pub opt_rtmp: bool,
    pub opt_normalize: bool,
    pub ffmpeg_path: String,
}

impl Config {
    pub fn new() -> Result<Config, String> {
        let matches = App::new(APP_NAME)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about("This program is a gadget which helps you use FFmpeg to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol. Your FFmpeg needs to enable libxcb, libfdk-aac and libx264 libraries.")
            .arg(Arg::with_name("w")
                .short("w")
                .long("window")
                .help("Selects a window to record.")
            )
            .arg(Arg::with_name("a")
                .short("a")
                .long("with-audio")
                .help("Records your screen with audio which could be internal or external. It depends on your computer environment.")
            )
            .arg(Arg::with_name("nn")
                .short("n")
                .long("no-normalize")
                .help("Does not pad the video size with black borders to the fixed ratio of 16:9.")
            )
            .arg(Arg::with_name("o")
                .short("o")
                .long("output")
                .help("Assigns a destination of your video. It should be a file path or a RTMP url.")
                .takes_value(true)
                .value_name("FILE/RTMP_URL")
            )
            .arg(Arg::with_name("ffmpeg")
                .short("ffmpeg")
                .long("ffmpeg-path")
                .help("Specifies the path of your FFmpeg executable binary file.")
                .takes_value(true)
                .value_name("FFMPEG_PATH")
                .default_value(DEFAULT_FFMPEG_PATH)
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let ffmpeg_path = matches.value_of("ffmpeg").unwrap();

        let ffmpeg_path = if ffmpeg_path.ne(DEFAULT_FFMPEG_PATH) {
            let mut path = Path::new(ffmpeg_path);

            let path = match path.canonicalize() {
                Ok(path) => {
                    path
                }
                Err(_) => {
                    return Err(String::from("FFMPEG_PATH is incorrect."));
                }
            };

            let path = path.to_str().unwrap();

            String::from(path)
        } else {
            String::from(ffmpeg_path)
        };

        let opt_window = matches.is_present("w");
        let opt_no_sound = !matches.is_present("a");
        let mut opt_rtmp = false;

        let opt_file_path = if let Some(t) = matches.value_of("o") {
            if t.starts_with("rtmp://") {
                opt_rtmp = true;
            }

            let path = Path::new(t);

            if let Err(err) = path.canonicalize() {
                if err.kind() != ErrorKind::NotFound {
                    return Err(format!("Unknown file path {:?}", err));
                }
            }

            String::from(t)
        } else {
            let utc: DateTime<Utc> = Utc::now();

            utc.format("%Y-%m-%d-%H-%M-%S.mp4").to_string()
        };

        let opt_normalize = !matches.is_present("nn");

        Ok(Config {
            opt_window,
            opt_no_sound,
            opt_file_path,
            opt_rtmp,
            opt_normalize,
            ffmpeg_path,
        })
    }
}

// TODO -----Config END-----

// TODO -----Resolution & Position START-----

pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

pub struct Position {
    pub x: i32,
    pub y: i32,
}

pub struct WindowInfo {
    pub screen: Resolution,
    pub resolution: Resolution,
    pub position: Position,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Position {
        Position { x, y }
    }
}

impl Resolution {
    pub fn new(width: i32, height: i32) -> Resolution {
        Resolution { width, height }
    }

    pub fn get_screen_resolution() -> Resolution {
        let res = {
            Exec::shell("xrandr")
                | Exec::shell("head -n 1")
                | Exec::shell("cut -d ',' -f 2")
                | Exec::shell("cut -d ' ' -f 3-5")
                | Exec::shell("tr -d ' '")
        }.capture()
            .unwrap()
            .stdout_str();

        let res: Vec<&str> = res.trim().split("x").collect();

        let res: Vec<i32> = res.iter().map(|&x| x.trim().parse().unwrap()).collect();

        Resolution::new(res[0], res[1])
    }

    pub fn get_normalized_resolution(&self) -> Resolution {
        let mut width = 7680;
        let mut height = 4320;

        for wh in [
            (3840, 2160),
            (2560, 1440),
            (1920, 1080),
            (1280, 720),
            (854, 480),
            (640, 360),
            (426, 240),
        ].iter()
            {
                if self.width <= wh.0 && self.height <= wh.1 {
                    width = wh.0;
                    height = wh.1;
                }
            }

        Resolution::new(width, height)
    }
}

impl WindowInfo {
    pub fn new() -> WindowInfo {
        let screen_resolution = Resolution::get_screen_resolution();

        let win_info = Exec::shell("xwininfo").capture().unwrap().stdout_str();

        let win_width: i32 = {
            Exec::shell("grep 'Width:'") | Exec::shell("cut -d ':' -f 2") | Exec::shell("tr -d ' '")
        }.stdin(win_info.as_str())
            .capture()
            .unwrap()
            .stdout_str()
            .trim()
            .parse()
            .unwrap();

        let win_height: i32 = {
            Exec::shell("grep 'Height:'")
                | Exec::shell("cut -d ':' -f 2")
                | Exec::shell("tr -d ' '")
        }.stdin(win_info.as_str())
            .capture()
            .unwrap()
            .stdout_str()
            .trim()
            .parse()
            .unwrap();

        let win_ux: i32 = {
            Exec::shell("grep 'Absolute upper-left X'")
                | Exec::shell("cut -d ':' -f 2")
                | Exec::shell("tr -d ' '")
        }.stdin(win_info.as_str())
            .capture()
            .unwrap()
            .stdout_str()
            .trim()
            .parse()
            .unwrap();

        let win_uy: i32 = {
            Exec::shell("grep 'Absolute upper-left Y'")
                | Exec::shell("cut -d ':' -f 2")
                | Exec::shell("tr -d ' '")
        }.stdin(win_info.as_str())
            .capture()
            .unwrap()
            .stdout_str()
            .trim()
            .parse()
            .unwrap();

        let width = if win_width + win_ux > screen_resolution.width {
            screen_resolution.width - win_ux
        } else {
            win_width
        };

        let height = if win_height + win_uy > screen_resolution.height {
            screen_resolution.height - win_uy
        } else {
            win_height
        };

        WindowInfo {
            screen: screen_resolution,
            resolution: Resolution::new(width, height),
            position: Position::new(win_ux, win_uy),
        }
    }
}

// TODO -----Resolution & Position END-----

pub fn try_delete_file(file_path: &str) {
    match fs::remove_file(file_path) {
        _ => {}
    }
}

pub fn get_number_of_processors() -> usize {
    num_cpus::get()
}

// TODO -----Test START-----

#[cfg(test)]
mod test {
    // use super::*;
}

// TODO -----Test END-----
