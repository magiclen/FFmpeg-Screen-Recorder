use std::borrow::Cow;
use std::error::Error;
use std::process;

use clap::{Arg, Command};
use terminal_size::terminal_size;

use chrono::prelude::*;

use nix::sys::signal;

use execute::{command_args, Execute};

use ffmpeg_screen_recorder::*;

const APP_NAME: &str = "FFmpeg Screen Recorder";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DEFAULT_FFMPEG_PATH: &str = "ffmpeg";

extern fn handle_sigint(_: i32) {
    eprintln!("Interrupted!");
}

#[inline]
fn handle_signals() {
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(handle_sigint),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    unsafe {
        signal::sigaction(signal::SIGINT, &sig_action).unwrap();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    handle_signals();

    let matches = Command::new(APP_NAME)
        .term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about("This program is a gadget which helps you use FFmpeg to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol. Your FFmpeg needs to enable libxcb, libfdk-aac and libx264 libraries.")
        .arg(Arg::new("w")
            .short('w')
            .long("window")
            .help("Select a window to record.")
        )
        .arg(Arg::new("a")
            .short('a')
            .long("with-audio")
            .help("Record your screen with audio which could be internal or external. It depends on your computer environment.")
        )
        .arg(Arg::new("nn")
            .short('n')
            .long("no-normalize")
            .help("Do not pad the video size with black borders to the fixed ratio of 16:9.")
        )
        .arg(Arg::new("o")
            .short('o')
            .long("output")
            .help("Assign a destination of your video. It should be a file path or a RTMP url.")
            .takes_value(true)
            .value_name("FILE/RTMP_URL")
        )
        .arg(Arg::new("ffmpeg")
            .short('f')
            .long("ffmpeg-path")
            .help("Specify the path of your FFmpeg executable binary file.")
            .takes_value(true)
            .value_name("FFMPEG_PATH")
            .default_value(DEFAULT_FFMPEG_PATH)
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches();

    let ffmpeg = matches.value_of("ffmpeg").unwrap();

    let opt_window = matches.is_present("w");
    let opt_no_sound = !matches.is_present("a");

    let (opt_rtmp, opt_file_path) = {
        match matches.value_of("o") {
            Some(t) => (t.starts_with("rtmp://"), Cow::from(t)),
            None => {
                let utc: DateTime<Utc> = Utc::now();

                (false, Cow::from(utc.format("%Y-%m-%d-%H-%M-%S.mp4").to_string()))
            }
        }
    };

    let opt_normalize = !matches.is_present("nn");

    if command_args!(ffmpeg, "-version").execute_check_exit_status_code(0).is_err() {
        return Err(format!("Cannot execute `{}`.", ffmpeg).into());
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

        if opt_no_sound {
            mute = ["-af", "volume=0"].as_ref();
        }
    } else if opt_no_sound {
        audio = ["-an"].as_ref();
    }

    let screen_resolution;
    let window_resolution;
    let position;
    let thread = (get_number_of_processors() / 2).max(1);

    if opt_window {
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
            x: 0,
            y: 0,
        };
    }

    let adjust_resolution = if opt_normalize {
        window_resolution.get_normalized_resolution()
    } else {
        Resolution {
            width: (window_resolution.width + 7) & !0b111,
            height: (window_resolution.height + 7) & !0b111,
        }
    };

    let thread_string = thread.to_string();
    let frame_rate_string = frame_rate.to_string();
    let res_str = format!("{}x{}", screen_resolution.width, screen_resolution.height);

    let mut command = command_args!(ffmpeg);

    command.args(&[
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

    let pad_arg = if opt_window {
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

    command.args(&["-vf", pad_arg.as_str()]);
    command.args(audio);
    command.args(mute);
    command.args(format);
    command.arg(opt_file_path.as_ref());

    let rtn = command.execute_output().map(|output| output.status.code())?;

    match rtn {
        Some(code) => {
            if code == 1 {
                try_delete_file(&opt_file_path);
            }
        }
        None => process::exit(1),
    }

    Ok(())
}
