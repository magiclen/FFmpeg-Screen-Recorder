use std::{
    borrow::Cow,
    fs,
    path::Path,
    process::{self, Command},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, anyhow};
use chrono::prelude::*;
use ffmpeg_screen_recorder::{
    Position, Resolution, audio,
    cli::{CLIArgs, get_args},
    ffmpeg::{self, Capabilities, Capture, RecordOptions},
    monitor::{self, Monitor},
    session,
    window_info::WindowInfo,
};
use nix::{
    libc,
    sys::signal::{SigHandler, Signal, signal},
};

/// FFmpeg exits with this code once it has finished writing the file after a signal.
const FFMPEG_INTERRUPTED_EXIT_CODE: i32 = 255;

/// Set when the user presses Ctrl-C, so that a deliberately stopped recording is not treated as a failure.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

fn main() -> anyhow::Result<()> {
    let mut args = get_args();

    if args.list_audio_devices {
        return audio::list_devices();
    }

    session::check_x11_session(args.display.is_some())?;

    let display = session::resolve_display(args.display.as_deref());

    let (rtmp, output) = match args.output.take() {
        Some(output) => {
            (output.starts_with("rtmp://") || output.starts_with("rtmps://"), Cow::from(output))
        },
        None => {
            let now: DateTime<Utc> = Utc::now();

            (false, Cow::from(now.format("%Y-%m-%d-%H-%M-%S.mp4").to_string()))
        },
    };

    let capabilities = Capabilities::probe(&args.ffmpeg_path)?;

    if let Some(hwaccel) = args.hwaccel {
        if !capabilities.has(hwaccel) {
            return Err(anyhow!(
                "{:?} was not built with the {} encoder",
                args.ffmpeg_path,
                hwaccel.encoder()
            ));
        }
    }

    // The GPU is looked up before anything is asked of the user, so that a missing one fails right away.
    let vaapi_device = match args.hwaccel {
        Some(ffmpeg::HwAccel::Vaapi) => Some(ffmpeg::find_vaapi_device()?),
        _ => None,
    };

    let (capture, pad_to) = select_capture(&args)?;

    let audio_device = if args.with_audio || args.audio_device.is_some() {
        Some(args.audio_device.as_deref().unwrap_or("default"))
    } else {
        None
    };

    let options = RecordOptions {
        display: display.as_str(),
        capture,
        draw_mouse: !args.no_mouse,
        frame_rate: args.fps.unwrap_or(if rtmp { 30 } else { 60 }),
        audio_device,
        audio_encoder: capabilities.audio_encoder(),
        duration: args.duration.as_deref(),
        pad_to,
        crf: args.crf.unwrap_or(if rtmp { 25 } else { 18 }),
        preset: args.preset.as_deref(),
        hwaccel: args.hwaccel,
        vaapi_device: vaapi_device.as_deref(),
        overwrite: args.overwrite,
        rtmp,
        output: output.as_ref(),
    };

    let ffmpeg_args = ffmpeg::build_args(&options);

    // Only a file that this run creates may be removed when FFmpeg fails.
    let created_by_this_run = !rtmp && !Path::new(output.as_ref()).exists();

    unsafe {
        // FFmpeg resets handled signals to their default on exec, so it still stops gracefully on Ctrl-C while this process stays alive to wait for it.
        signal(Signal::SIGINT, SigHandler::Handler(handle_sigint))
            .context("cannot install the SIGINT handler")?;
    }

    let status = Command::new(&args.ffmpeg_path)
        .args(&ffmpeg_args)
        .status()
        .with_context(|| anyhow!("cannot execute {:?}", args.ffmpeg_path))?;

    let code = status.code().unwrap_or(1);

    if code == 0 || INTERRUPTED.load(Ordering::SeqCst) || code == FFMPEG_INTERRUPTED_EXIT_CODE {
        return Ok(());
    }

    if created_by_this_run {
        let _ = fs::remove_file(output.as_ref());
    }

    process::exit(code);
}

/// Work out which rectangle to capture, and how big the padded output has to be.
fn select_capture(args: &CLIArgs) -> anyhow::Result<(Capture, Option<Resolution>)> {
    if args.select_region {
        return Ok((Capture::SelectRegion, None));
    }

    let monitors = Monitor::list()?;
    let screen = monitor::bounding_box(&monitors);

    let (position, resolution) = if args.window {
        eprintln!("Please select a window with your mouse.");

        let window = WindowInfo::select()?.clip_to(screen)?;

        (window.position, window.resolution)
    } else if let Some(name) = args.monitor.as_deref() {
        let monitor = monitors.iter().find(|monitor| monitor.name == name).ok_or_else(|| {
            anyhow!(
                "there is no monitor named {name:?}; the available ones are {}",
                monitors.iter().map(|monitor| monitor.name.as_str()).collect::<Vec<_>>().join(", ")
            )
        })?;

        (monitor.position, monitor.resolution)
    } else {
        (Position::new(0, 0), screen)
    };

    if args.normalize {
        let pad_to = resolution.get_normalized_resolution().ok_or_else(|| {
            anyhow!(
                "{}x{} is larger than 8K, so it cannot be padded to a 16:9 size",
                resolution.width,
                resolution.height
            )
        })?;

        return Ok((
            Capture::Region {
                position,
                resolution,
            },
            Some(pad_to),
        ));
    }

    // yuv420p needs even dimensions, and dropping at most one pixel beats adding a black line.
    Ok((
        Capture::Region {
            position,
            resolution: resolution.to_even(),
        },
        None,
    ))
}
