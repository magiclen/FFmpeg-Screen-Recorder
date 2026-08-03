use std::{
    fs,
    os::unix::process::CommandExt,
    path::Path,
    process::{self, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicI32, Ordering},
    thread,
    time::Duration,
};

use anyhow::{Context, anyhow};
use chrono::prelude::*;
use ffmpeg_screen_recorder::{
    Position, Resolution, audio,
    cli::{CLIArgs, get_args},
    ffmpeg::{
        self, AudioCapture, Capabilities, Capture, NvencPreset, RecordOptions, SoftwarePreset,
        VideoEncoder,
    },
    monitor::{self, Monitor},
    session,
    window_info::WindowInfo,
};
use nix::{
    errno::Errno,
    libc,
    sys::signal::{SigHandler, Signal, kill, signal},
    unistd::Pid,
};
use tempfile::{Builder as TempFileBuilder, NamedTempFile};

/// FFmpeg exits with this code once it has finished writing the file after a signal.
const FFMPEG_INTERRUPTED_EXIT_CODE: i32 = 255;

/// The signal waiting to be forwarded to FFmpeg.
static RECEIVED_SIGNAL: AtomicI32 = AtomicI32::new(0);

extern "C" fn handle_signal(signal: libc::c_int) {
    RECEIVED_SIGNAL.store(signal, Ordering::SeqCst);
}

fn main() -> anyhow::Result<()> {
    let mut args = get_args();

    if args.list_audio_devices {
        return audio::list_devices();
    }

    session::check_x11_session(args.display.is_some())?;

    let display = session::resolve_display(args.display.as_deref());

    let (rtmp, output) = match args.output.take() {
        Some(output) => (output.starts_with("rtmp://") || output.starts_with("rtmps://"), output),
        None => {
            let now: DateTime<Utc> = Utc::now();

            (false, now.format("%Y-%m-%d-%H-%M-%S.mp4").to_string())
        },
    };

    let temporary_output =
        if rtmp { None } else { Some(create_temporary_output(&output, args.overwrite)?) };
    let ffmpeg_output = temporary_output
        .as_ref()
        .map(|file| file.path().to_string_lossy().into_owned())
        .unwrap_or_else(|| output.clone());

    let capabilities = Capabilities::probe(&args.ffmpeg_path)?;

    let audio_capture = match (args.with_audio, args.microphone, args.audio_device.as_deref()) {
        (true, true, None) => AudioCapture::Mix {
            system:     "@DEFAULT_MONITOR@",
            microphone: "@DEFAULT_SOURCE@",
        },
        (true, false, None) => AudioCapture::Device("@DEFAULT_MONITOR@"),
        (false, true, None) => AudioCapture::Device("@DEFAULT_SOURCE@"),
        (false, false, Some(device)) => AudioCapture::Device(device),
        (false, false, None) => AudioCapture::None,
        _ => unreachable!("clap rejects --audio-device with other audio source options"),
    };

    capabilities.validate(&args.ffmpeg_path, args.hwaccel, audio_capture, rtmp)?;

    let automatic_vaapi_device = match (args.hwaccel, args.vaapi_device.as_deref()) {
        (Some(ffmpeg::HwAccel::Vaapi), None) => Some(ffmpeg::find_vaapi_device()?),
        _ => None,
    };

    let quality = args.quality.unwrap_or(if rtmp { 25 } else { 18 });
    let video_encoder = match args.hwaccel {
        Some(ffmpeg::HwAccel::Vaapi) => {
            if args.nvenc_gpu.is_some() {
                return Err(anyhow!("--nvenc-gpu requires --hwaccel nvenc"));
            }
            if let Some(preset) = args.preset.as_deref() {
                return Err(anyhow!(
                    "--preset {preset:?} cannot be used with VAAPI; the driver controls its speed \
                     and quality tradeoff"
                ));
            }

            let device = args
                .vaapi_device
                .as_deref()
                .or(automatic_vaapi_device.as_deref())
                .expect("a VAAPI device was provided or found");

            VideoEncoder::Vaapi {
                device,
                qp: quality,
            }
        },
        Some(ffmpeg::HwAccel::Nvenc) => {
            if args.vaapi_device.is_some() {
                return Err(anyhow!("--vaapi-device requires --hwaccel vaapi"));
            }

            let preset = args
                .preset
                .as_deref()
                .unwrap_or("p4")
                .parse::<NvencPreset>()
                .map_err(anyhow::Error::msg)?;

            VideoEncoder::Nvenc {
                gpu: args.nvenc_gpu,
                qp: quality,
                preset,
            }
        },
        None => {
            if args.vaapi_device.is_some() {
                return Err(anyhow!("--vaapi-device requires --hwaccel vaapi"));
            }
            if args.nvenc_gpu.is_some() {
                return Err(anyhow!("--nvenc-gpu requires --hwaccel nvenc"));
            }

            let preset = args
                .preset
                .as_deref()
                .unwrap_or("veryfast")
                .parse::<SoftwarePreset>()
                .map_err(anyhow::Error::msg)?;

            VideoEncoder::Libx264 {
                crf: quality,
                preset,
            }
        },
    };

    let (capture, pad_to) = select_capture(&args, &display)?;

    let options = RecordOptions {
        display: display.as_str(),
        capture,
        draw_mouse: !args.no_mouse,
        frame_rate: args.fps.unwrap_or(if rtmp { 30 } else { 60 }),
        audio_capture,
        audio_encoder: capabilities.audio_encoder(),
        duration: args.duration.as_ref().map(|duration| duration.as_str()),
        pad_to,
        video_encoder,
        overwrite: temporary_output.is_some() || args.overwrite,
        rtmp,
        output: &ffmpeg_output,
    };

    let ffmpeg_args = ffmpeg::build_args(&options);

    unsafe {
        signal(Signal::SIGINT, SigHandler::Handler(handle_signal))
            .context("cannot install the SIGINT handler")?;
        signal(Signal::SIGTERM, SigHandler::Handler(handle_signal))
            .context("cannot install the SIGTERM handler")?;
    }

    let mut child = Command::new(&args.ffmpeg_path)
        .args(&ffmpeg_args)
        .stdin(Stdio::null())
        .process_group(0)
        .spawn()
        .with_context(|| anyhow!("cannot execute {:?}", args.ffmpeg_path))?;
    let (status, interrupted) = wait_for_ffmpeg(&mut child)?;

    let code = status.code().unwrap_or(1);

    if code == 0 || interrupted || code == FFMPEG_INTERRUPTED_EXIT_CODE {
        if let Some(temporary_output) = temporary_output {
            publish_output(temporary_output, &output, args.overwrite)?;
        }

        return Ok(());
    }

    drop(temporary_output);

    process::exit(code);
}

fn wait_for_ffmpeg(child: &mut process::Child) -> anyhow::Result<(ExitStatus, bool)> {
    let mut interrupted = false;

    loop {
        let received_signal = RECEIVED_SIGNAL.swap(0, Ordering::SeqCst);

        if received_signal != 0 {
            let forwarded_signal = match received_signal {
                libc::SIGINT => Signal::SIGINT,
                libc::SIGTERM => Signal::SIGTERM,
                _ => unreachable!("only SIGINT and SIGTERM are handled"),
            };

            interrupted = true;

            match kill(Pid::from_raw(child.id() as i32), forwarded_signal) {
                Ok(()) | Err(Errno::ESRCH) => {},
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("cannot forward {forwarded_signal:?} to FFmpeg"));
                },
            }
        }

        if let Some(status) = child.try_wait().context("cannot wait for FFmpeg")? {
            return Ok((status, interrupted));
        }

        thread::sleep(Duration::from_millis(25));
    }
}

fn create_temporary_output(output: &str, overwrite: bool) -> anyhow::Result<NamedTempFile> {
    let output = Path::new(output);

    if !overwrite && output.exists() {
        return Err(anyhow!(
            "the output file {output:?} already exists; pass --overwrite to replace it"
        ));
    }

    let parent =
        output.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let suffix = output.extension().map(|extension| format!(".{}", extension.to_string_lossy()));
    let mut builder = TempFileBuilder::new();

    builder.prefix(".ffmpeg-screen-recorder-");
    if let Some(suffix) = suffix.as_deref() {
        builder.suffix(suffix);
    }

    builder
        .tempfile_in(parent)
        .with_context(|| anyhow!("cannot create a temporary output file in {parent:?}"))
}

fn publish_output(
    temporary_output: NamedTempFile,
    output: &str,
    overwrite: bool,
) -> anyhow::Result<()> {
    let output = Path::new(output);

    if overwrite {
        return temporary_output.persist(output).map(|_| ()).with_context(|| {
            anyhow!("the recording was finished but cannot be moved to {output:?}")
        });
    }

    match fs::hard_link(temporary_output.path(), output) {
        Ok(()) => temporary_output.close().with_context(|| {
            anyhow!(
                "the recording was saved to {output:?}, but its temporary name cannot be removed"
            )
        }),
        Err(error) => {
            let temporary_path = temporary_output.path().to_path_buf();

            match temporary_output.keep() {
                Ok(_) => Err(anyhow!(
                    "the recording was finished but cannot be moved to {output:?}: {error}; it \
                     remains at {temporary_path:?}"
                )),
                Err(keep_error) => Err(anyhow!(
                    "the recording was finished but cannot be moved to {output:?}: {error}; its \
                     temporary file also cannot be preserved: {keep_error}"
                )),
            }
        },
    }
}

/// Work out which rectangle to capture, and how big the padded output has to be.
fn select_capture(args: &CLIArgs, display: &str) -> anyhow::Result<(Capture, Option<Resolution>)> {
    if args.select_region {
        return Ok((Capture::SelectRegion, None));
    }

    let monitors = Monitor::list(display)?;
    let screen = monitor::bounding_box(&monitors);

    let (position, resolution) = if args.window {
        eprintln!("Please select a window with your mouse.");

        let window = WindowInfo::select(display)?.clip_to(screen)?;

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
                "{}x{} is larger than 16K, so it cannot be normalized",
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

#[cfg(test)]
mod tests {
    use std::{io::Write, os::unix::process::ExitStatusExt};

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn signal_is_forwarded_to_ffmpeg() {
        let mut child = Command::new("sh")
            .args(["-c", "while :; do :; done"])
            .process_group(0)
            .spawn()
            .unwrap();

        RECEIVED_SIGNAL.store(libc::SIGTERM, Ordering::SeqCst);

        let (status, interrupted) = wait_for_ffmpeg(&mut child).unwrap();

        assert!(interrupted);
        assert_eq!(Some(libc::SIGTERM), status.signal());
    }

    #[test]
    fn publishing_without_overwrite_moves_the_recording() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("recording.mp4");
        let mut temporary_output =
            TempFileBuilder::new().suffix(".mp4").tempfile_in(directory.path()).unwrap();
        let temporary_path = temporary_output.path().to_path_buf();

        temporary_output.write_all(b"recording").unwrap();

        publish_output(temporary_output, output.to_str().unwrap(), false).unwrap();

        assert_eq!(b"recording", fs::read(&output).unwrap().as_slice());
        assert!(!temporary_path.exists());
    }

    #[test]
    fn publishing_without_overwrite_preserves_an_existing_output() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("recording.mp4");
        let mut temporary_output =
            TempFileBuilder::new().suffix(".mp4").tempfile_in(directory.path()).unwrap();
        let temporary_path = temporary_output.path().to_path_buf();

        temporary_output.write_all(b"recording").unwrap();
        fs::write(&output, b"other process").unwrap();

        assert!(publish_output(temporary_output, output.to_str().unwrap(), false).is_err());
        assert_eq!(b"other process", fs::read(&output).unwrap().as_slice());
        assert_eq!(b"recording", fs::read(temporary_path).unwrap().as_slice());
    }
}
