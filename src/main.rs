extern crate ffmpeg_screen_recorder;
extern crate nix;
extern crate subprocess;

use ffmpeg_screen_recorder::*;
use std::cmp;
use std::process;

use subprocess::{Exec, ExitStatus};

use nix::sys::signal;

extern "C" fn handle_sigint(_: i32) {
    eprintln!("Interrupted!");
}

fn main() {
    unsafe {
        let sig_action = signal::SigAction::new(
            signal::SigHandler::Handler(handle_sigint),
            signal::SaFlags::empty(),
            signal::SigSet::empty(),
        );
        signal::sigaction(signal::SIGINT, &sig_action).unwrap();
    }

    let config = Config::new();

    match config {
        Ok(config) => {
            if let Err(_) = Exec::cmd(&config.ffmpeg_path).capture() {
                eprintln!("FFMPEG_PATH is incorrect or the file cannot be executed.");
                process::exit(1);
            }

            let mut video = vec![
                "-vcodec", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-crf", "18",
            ];
            let mut audio = vec!["-acodec", "libfdk_aac", "-vbr", "5", "-ar", "44100"];
            let mut mute = vec![];
            let mut frame_rate = 60;
            let mut format = vec![];

            if config.opt_rtmp {
                frame_rate = 30;
                format = vec!["-f", "flv"];
                video = vec![
                    "-vcodec", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-crf",
                    "25",
                ];
                if config.opt_no_sound {
                    mute = vec!["-af", "volume=0"];
                }
            } else {
                if config.opt_no_sound {
                    audio = vec!["-an"];
                }
            }

            let screen_resolution;
            let window_resolution;
            let position;
            let thread = cmp::max(get_number_of_processors() / 2, 1);

            if config.opt_window {
                eprintln!("Please select a window with your mouse.");

                let window_info = WindowInfo::new();
                let res = window_info.resolution;
                let pos = window_info.position;
                let screen = window_info.screen;

                screen_resolution = screen;
                window_resolution = res;
                position = pos;
            } else {
                let res = Resolution::get_screen_resolution();

                screen_resolution = Resolution { ..res };
                window_resolution = res;
                position = Position { x: 0, y: 0 };
            }

            let adjust_resolution = if config.opt_normalize {
                window_resolution.get_normalized_resolution()
            } else {
                Resolution {
                    width: (window_resolution.width + 7) / 8 * 8,
                    height: (window_resolution.height + 7) / 8 * 8,
                }
            };

            let mut process = Exec::cmd(&config.ffmpeg_path)
                .arg("-threads")
                .arg(thread.to_string())
                .arg("-f")
                .arg("x11grab")
                .arg("-r")
                .arg(frame_rate.to_string())
                .arg("-s")
                .arg(format!(
                    "{}x{}",
                    screen_resolution.width, screen_resolution.height
                ))
                .arg("-i")
                .arg(":0")
                .arg("-f")
                .arg("pulse")
                .arg("-ac")
                .arg("2")
                .arg("-i")
                .arg("default")
                .args(&video)
                .arg("-vf");

            if config.opt_window {
                process = process.arg(format!(
                    "crop={}:{}:{}:{},pad={}:{}:(ow-iw)/2:(oh-ih)/2",
                    window_resolution.width,
                    window_resolution.height,
                    position.x,
                    position.y,
                    adjust_resolution.width,
                    adjust_resolution.height
                ));
            } else {
                process = process.arg(format!(
                    "pad={}:{}:(ow-iw)/2:(oh-ih)/2",
                    adjust_resolution.width, adjust_resolution.height
                ));
            }

            let process = process
                .args(&audio)
                .args(&mute)
                .args(&format)
                .arg(&config.opt_file_path);

            if let Ok(e) = process.join() {
                if let ExitStatus::Exited(exit) = e {
                    if exit == 1 {
                        try_delete_file(&config.opt_file_path);
                    }
                }
            }
        }
        Err(s) => {
            eprintln!("{}", s);
        }
    }
}
