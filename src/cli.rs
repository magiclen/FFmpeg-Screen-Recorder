use std::{fmt, str::FromStr};

use clap::{CommandFactory, FromArgMatches, Parser};
use concat_with::concat_line;
use terminal_size::terminal_size;

use crate::ffmpeg::HwAccel;

const APP_NAME: &str = "FFmpeg Screen Recorder";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "This program is a gadget which helps you use FFmpeg to record your screen on Linux (X11 \
     only). The video record can be saved as a file, or be streamed via RTMP protocol. FFmpeg 8 \
     or newer with libxcb and a supported H.264 encoder is expected; libfdk-aac is used when it \
     is available.\n\nEXAMPLES:\n",
    concat_line!(prefix "ffmpeg-screen-recorder ",
        "                        # Record every monitor without audio into the current working directory",
        "-w                      # Select a window with the mouse and record it",
        "--select-region         # Drag a rectangle with the mouse and record it",
        "-m HDMI-0               # Record a single monitor",
        "-a                      # Record with the system audio",
        "--microphone            # Record with the default microphone",
        "-a --microphone         # Mix the system audio and default microphone",
        "-t 30 -o /path/to/file  # Record for 30 seconds and output to /path/to/file",
        "-o rtmp://xxx           # Stream to an RTMP url",
        "--hwaccel vaapi         # Let the GPU encode instead of the CPU",
    )
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duration(String);

impl Duration {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Duration {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for Duration {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid = if value.contains(':') {
            parse_clock_duration(value)
        } else {
            parse_seconds_duration(value)
        };

        if valid {
            Ok(Duration(value.to_string()))
        } else {
            Err(String::from(
                "duration must be positive seconds, optionally ending in s, ms, or us, or \
                 MM:SS[.fraction] or HH:MM:SS[.fraction]",
            ))
        }
    }
}

fn parse_clock_duration(value: &str) -> bool {
    let parts: Vec<&str> = value.split(':').collect();

    if !(2..=3).contains(&parts.len()) {
        return false;
    }

    let seconds = parts[parts.len() - 1];
    let minutes = parts[parts.len() - 2];

    if !parse_bounded_integer(minutes, 59) || !parse_bounded_decimal(seconds, 59) {
        return false;
    }

    if parts.len() == 3 && (!is_digits(parts[0]) || parts[0].len() > 2) {
        return false;
    }

    parts.iter().any(|part| part.bytes().any(|byte| byte.is_ascii_digit() && byte != b'0'))
}

fn parse_seconds_duration(value: &str) -> bool {
    let number = value
        .strip_suffix("ms")
        .or_else(|| value.strip_suffix("us"))
        .or_else(|| value.strip_suffix('s'))
        .unwrap_or(value);

    parse_decimal(number) && number.bytes().any(|byte| byte.is_ascii_digit() && byte != b'0')
}

fn parse_bounded_integer(value: &str, maximum: u8) -> bool {
    !value.is_empty()
        && value.len() <= 2
        && is_digits(value)
        && value.parse::<u8>().is_ok_and(|value| value <= maximum)
}

fn parse_bounded_decimal(value: &str, maximum: u8) -> bool {
    let integer = value.split_once('.').map_or(value, |(integer, _)| integer);

    parse_decimal(value) && parse_bounded_integer(integer, maximum)
}

fn parse_decimal(value: &str) -> bool {
    match value.split_once('.') {
        Some((integer, fraction)) => {
            !integer.is_empty() && !fraction.is_empty() && is_digits(integer) && is_digits(fraction)
        },
        None => is_digits(value),
    }
}

#[inline]
fn is_digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug, Parser)]
#[command(name = APP_NAME)]
#[command(term_width = terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))]
#[command(version = CARGO_PKG_VERSION)]
#[command(author = CARGO_PKG_AUTHORS)]
#[command(after_help = AFTER_HELP)]
pub struct CLIArgs {
    #[arg(short, long)]
    #[arg(conflicts_with_all = ["select_region", "monitor"])]
    #[arg(help = "Select a window to record")]
    pub window: bool,

    #[arg(long)]
    #[arg(conflicts_with = "monitor")]
    #[arg(help = "Drag a rectangle with your mouse and record only that area")]
    pub select_region: bool,

    #[arg(short, long, value_name = "MONITOR")]
    #[arg(
        help = "Record one monitor, named as in `xrandr --listmonitors` [default: every monitor]"
    )]
    pub monitor: Option<String>,

    #[arg(short = 'a', long)]
    #[arg(help = "Record the sound playing through the default output device")]
    pub with_audio: bool,

    #[arg(long)]
    #[arg(help = "Record from the default microphone; combine it with --with-audio to mix both")]
    pub microphone: bool,

    #[arg(long, value_name = "DEVICE")]
    #[arg(conflicts_with_all = ["with_audio", "microphone"])]
    #[arg(help = "Record only from this PulseAudio source")]
    pub audio_device: Option<String>,

    #[arg(long)]
    #[arg(exclusive = true)]
    #[arg(help = "List the PulseAudio sources that --audio-device accepts, then exit")]
    pub list_audio_devices: bool,

    #[arg(long)]
    #[arg(conflicts_with = "select_region")]
    #[arg(help = "Pad the capture to the smallest fitting common 16:9 resolution, up to 16K")]
    pub normalize: bool,

    #[arg(long)]
    #[arg(help = "Do not record the mouse pointer")]
    pub no_mouse: bool,

    #[arg(long, value_name = "FPS")]
    #[arg(value_parser = clap::value_parser!(u32).range(1..=240))]
    #[arg(help = "Set the frame rate [default: 60, or 30 when streaming]")]
    pub fps: Option<u32>,

    #[arg(long, alias = "crf", value_name = "QUALITY")]
    #[arg(value_parser = clap::value_parser!(u8).range(0..=51))]
    #[arg(help = "Set recording quality; a smaller number means a bigger file [default: 18, or \
                  25 when streaming]")]
    pub quality: Option<u8>,

    #[arg(long, value_name = "PRESET")]
    #[arg(help = "Choose a libx264 or NVENC preset; VAAPI does not support presets")]
    pub preset: Option<String>,

    #[arg(long, value_name = "BACKEND")]
    #[arg(value_enum)]
    #[arg(help = "Let the GPU encode the video instead of the CPU")]
    pub hwaccel: Option<HwAccel>,

    #[arg(long, value_name = "PATH")]
    #[arg(conflicts_with = "nvenc_gpu")]
    #[arg(help = "Select a DRM render node for --hwaccel vaapi [default: first render node]")]
    pub vaapi_device: Option<String>,

    #[arg(long, value_name = "INDEX")]
    #[arg(conflicts_with = "vaapi_device")]
    #[arg(help = "Select a GPU index for --hwaccel nvenc [default: first capable GPU]")]
    pub nvenc_gpu: Option<u32>,

    #[arg(short = 't', long, value_name = "DURATION")]
    #[arg(help = "Stop after this long, as seconds, MM:SS, or HH:MM:SS")]
    pub duration: Option<Duration>,

    #[arg(short = 'y', long)]
    #[arg(help = "Overwrite the output file if it already exists")]
    pub overwrite: bool,

    #[arg(short, long, value_name = "DISPLAY")]
    #[arg(help = "Record this X display [default: $DISPLAY]")]
    pub display: Option<String>,

    #[arg(short, long, value_name = "FILE/RTMP_URL")]
    #[arg(value_hint = clap::ValueHint::FilePath)]
    #[arg(help = "Assign a destination of your video. It should be a file path or a RTMP url \
                  [default: CWD/<time>.mp4]")]
    pub output: Option<String>,

    #[arg(short, long)]
    #[arg(default_value = "ffmpeg")]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your FFmpeg executable binary file")]
    pub ffmpeg_path: String,
}

pub fn get_args() -> CLIArgs {
    let args = CLIArgs::command();

    let about = format!("{APP_NAME} {CARGO_PKG_VERSION}\n{CARGO_PKG_AUTHORS}\n{APP_ABOUT}");

    let args = args.about(about);

    let matches = args.get_matches();

    match CLIArgs::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(err) => {
            err.exit();
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_accepts_supported_formats() {
        for duration in ["30", "1.5", "200ms", "200000us", "01:30", "12:03:45.5"] {
            assert_eq!(duration, duration.parse::<Duration>().unwrap().as_str());
        }
    }

    #[test]
    fn duration_rejects_non_positive_or_invalid_formats() {
        for duration in ["0", "0.0", "-1", "1m", "1:60", "1:2:3:4", "text"] {
            assert!(duration.parse::<Duration>().is_err(), "{duration:?} should be rejected");
        }
    }
}
