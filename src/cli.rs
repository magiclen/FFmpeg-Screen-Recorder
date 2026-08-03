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
     or newer with libxcb and libx264 is expected; libfdk-aac is used when it is \
     available.\n\nEXAMPLES:\n",
    concat_line!(prefix "ffmpeg-screen-recorder ",
        "                        # Record every monitor without audio into the current working directory",
        "-w                      # Select a window with the mouse and record it",
        "--select-region         # Drag a rectangle with the mouse and record it",
        "-m HDMI-0               # Record a single monitor",
        "-a                      # Record with the system audio",
        "-t 30 -o /path/to/file  # Record for 30 seconds and output to /path/to/file",
        "-o rtmp://xxx           # Stream to an RTMP url",
        "--hwaccel vaapi         # Let the GPU encode instead of the CPU",
    )
);

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
    #[arg(help = "Record your screen with audio which could be internal or external. It depends \
                  on your computer environment")]
    pub with_audio: bool,

    #[arg(long, value_name = "DEVICE")]
    #[arg(help = "Record from this PulseAudio source, which also implies --with-audio [default: \
                  default]")]
    pub audio_device: Option<String>,

    #[arg(long)]
    #[arg(exclusive = true)]
    #[arg(help = "List the PulseAudio sources that --audio-device accepts, then exit")]
    pub list_audio_devices: bool,

    #[arg(long)]
    #[arg(conflicts_with = "select_region")]
    #[arg(help = "Pad the video size with black borders to the fixed ratio of 16:9")]
    pub normalize: bool,

    #[arg(long)]
    #[arg(help = "Do not record the mouse pointer")]
    pub no_mouse: bool,

    #[arg(long, value_name = "FPS")]
    #[arg(value_parser = clap::value_parser!(u32).range(1..=240))]
    #[arg(help = "Set the frame rate [default: 60, or 30 when streaming]")]
    pub fps: Option<u32>,

    #[arg(long, value_name = "CRF")]
    #[arg(value_parser = clap::value_parser!(u8).range(0..=51))]
    #[arg(help = "Set the quality, where a smaller number means a bigger file [default: 18, or \
                  25 when streaming]")]
    pub crf: Option<u8>,

    #[arg(long, value_name = "PRESET")]
    #[arg(help = "Set the encoder preset, such as ultrafast or slow [default: veryfast]")]
    pub preset: Option<String>,

    #[arg(long, value_name = "BACKEND")]
    #[arg(value_enum)]
    #[arg(help = "Let the GPU encode the video instead of the CPU")]
    pub hwaccel: Option<HwAccel>,

    #[arg(short = 't', long, value_name = "DURATION")]
    #[arg(help = "Stop after this long, as seconds or as HH:MM:SS")]
    pub duration: Option<String>,

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
