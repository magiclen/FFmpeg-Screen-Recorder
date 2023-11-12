mod cli;
mod position;
mod resolution;
mod window_info;

use std::{borrow::Cow, fs, process};

use anyhow::anyhow;
use chrono::prelude::*;
use cli::*;
use execute::{command_args, Execute};
use position::Position;
use resolution::Resolution;
use window_info::WindowInfo;

fn main() -> anyhow::Result<()> {
    let args = get_args();

    let (opt_rtmp, opt_file_path) = {
        match args.output {
            Some(t) => (t.starts_with("rtmp://"), Cow::from(t)),
            None => {
                let utc: DateTime<Utc> = Utc::now();

                (false, Cow::from(utc.format("%Y-%m-%d-%H-%M-%S.mp4").to_string()))
            },
        }
    };

    if command_args!(&args.ffmpeg_path, "-version").execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot execute {:?}.", args.ffmpeg_path));
    }

    let mut video =
        ["-vcodec", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-crf", "18"].as_ref();
    let mut audio = ["-acodec", "libfdk_aac", "-vbr", "5", "-ar", "44100"].as_ref();
    let mut mute = [].as_ref();
    let mut frame_rate = 60;
    let mut format = [].as_ref();

    if opt_rtmp {
        frame_rate = 30;
        format = ["-f", "flv"].as_ref();
        video = ["-vcodec", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-crf", "25"]
            .as_ref();

        if !args.with_audio {
            mute = ["-af", "volume=0"].as_ref();
        }
    } else if !args.with_audio {
        audio = ["-an"].as_ref();
    }

    let screen_resolution;
    let window_resolution;
    let position;
    let thread = (num_cpus::get() / 2).max(1);

    if args.window {
        eprintln!("Please select a window with your mouse.");

        let window_info = WindowInfo::new()?;
        let res = window_info.resolution;
        let pos = window_info.position;
        let screen = window_info.screen;

        screen_resolution = screen;
        window_resolution = res;
        position = pos;
    } else {
        let res = Resolution::get_screen_resolution()?;

        screen_resolution = Resolution {
            ..res
        };
        window_resolution = res;
        position = Position {
            x: 0, y: 0
        };
    }

    let adjust_resolution = if args.no_normalize {
        Resolution {
            width:  (window_resolution.width + 7) & !0b111,
            height: (window_resolution.height + 7) & !0b111,
        }
    } else {
        window_resolution.get_normalized_resolution()
    };

    let thread_string = thread.to_string();
    let frame_rate_string = frame_rate.to_string();
    let res_str = format!("{}x{}", screen_resolution.width, screen_resolution.height);

    let mut command = command_args!(args.ffmpeg_path);

    command.args([
        "-threads",
        thread_string.as_str(),
        "-f",
        "x11grab",
        "-r",
        frame_rate_string.as_str(),
        "-s",
        res_str.as_str(),
        "-i",
        ":0",
        "-f",
        "pulse",
        "-ac",
        "2",
        "-i",
        "default",
    ]);

    command.args(video);

    let pad_arg = if args.window {
        format!(
            "crop={}:{}:{}:{},pad={}:{}:(ow-iw)/2:(oh-ih)/2",
            window_resolution.width,
            window_resolution.height,
            position.x,
            position.y,
            adjust_resolution.width,
            adjust_resolution.height
        )
    } else {
        format!("pad={}:{}:(ow-iw)/2:(oh-ih)/2", adjust_resolution.width, adjust_resolution.height)
    };

    command.args(["-vf", pad_arg.as_str()]);
    command.args(audio);
    command.args(mute);
    command.args(format);
    command.arg(opt_file_path.as_ref());

    let rtn = command.execute_output().map(|output| output.status.code())?;

    match rtn {
        Some(code) => {
            if code == 1 {
                let _ = fs::remove_file(opt_file_path.as_ref());
            }
        },
        None => process::exit(1),
    }

    Ok(())
}
