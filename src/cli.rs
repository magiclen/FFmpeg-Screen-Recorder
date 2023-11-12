use clap::{CommandFactory, FromArgMatches, Parser};
use concat_with::concat_line;
use terminal_size::terminal_size;

const APP_NAME: &str = "FFmpeg Screen Recorder";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "This program is a gadget which helps you use FFmpeg to record your screen on Linux. The \
     video record can be saved as a file, or be streamed via RTMP protocol. Your FFmpeg needs to \
     enable libxcb, libfdk-aac and libx264 libraries.\n\nEXAMPLES:\n",
    concat_line!(prefix "ffmpeg-screen-recorder ",
        "                   # Record the full screen without audio and output into the current working directory",
        "-w                 # Select a window and record it without audio and output into the current working directory",
        "-a                 # Record the full screen with the system audio and output into the current working directory",
        "-o /path/to/file   # Record the full screen without audio and output to /path/to/file",
        "-o rtmp://xxx      # Record the full screen without audio and output to rtmp://xxx",
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
    #[arg(help = "Select a window to record")]
    pub window:       bool,
    #[arg(short = 'a', long)]
    #[arg(help = "Record your screen with audio which could be internal or external. It depends \
                  on your computer environment")]
    pub with_audio:   bool,
    #[arg(short, long, visible_alias = "nn")]
    #[arg(help = "Do not pad the video size with black borders to the fixed ratio of 16:9")]
    pub no_normalize: bool,
    #[arg(short, long, value_name = "FILE/RTMP_URL")]
    #[arg(value_hint = clap::ValueHint::FilePath)]
    #[arg(help = "Assign a destination of your video. It should be a file path or a RTMP url \
                  [default: CWD/<time>.mp4]")]
    pub output:       Option<String>,
    #[arg(short, long)]
    #[arg(default_value = "ffmpeg")]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your FFmpeg executable binary file")]
    pub ffmpeg_path:  String,
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
