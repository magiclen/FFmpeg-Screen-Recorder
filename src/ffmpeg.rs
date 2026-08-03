use std::{fs, process::Command};

use anyhow::{Context, anyhow};

use crate::{position::Position, resolution::Resolution};

/// The video encoder backend, which decides whether the CPU or the GPU does the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum HwAccel {
    /// Intel and AMD GPUs, through `h264_vaapi`.
    Vaapi,
    /// NVIDIA GPUs, through `h264_nvenc`.
    Nvenc,
}

impl HwAccel {
    #[inline]
    pub const fn encoder(self) -> &'static str {
        match self {
            HwAccel::Vaapi => "h264_vaapi",
            HwAccel::Nvenc => "h264_nvenc",
        }
    }
}

/// The AAC encoder to use for the audio track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEncoder {
    /// Better quality, but only present in FFmpeg builds configured with `--enable-nonfree`.
    LibfdkAac,
    /// The encoder built into every FFmpeg.
    Aac,
}

/// Which encoders the FFmpeg being used was built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub libfdk_aac: bool,
    pub h264_vaapi: bool,
    pub h264_nvenc: bool,
}

impl Capabilities {
    /// Run FFmpeg once, both to check that it works and to see which encoders it has.
    pub fn probe(ffmpeg_path: &str) -> anyhow::Result<Capabilities> {
        let output = Command::new(ffmpeg_path)
            .args(["-hide_banner", "-encoders"])
            .output()
            .with_context(|| anyhow!("cannot execute {ffmpeg_path:?}"))?;

        if !output.status.success() {
            return Err(anyhow!(
                "{ffmpeg_path:?} cannot list its encoders; FFmpeg 8 or newer is expected"
            ));
        }

        let encoders = String::from_utf8_lossy(&output.stdout);

        Ok(Capabilities {
            libfdk_aac: encoders.contains("libfdk_aac"),
            h264_vaapi: encoders.contains("h264_vaapi"),
            h264_nvenc: encoders.contains("h264_nvenc"),
        })
    }

    #[inline]
    pub const fn has(&self, hwaccel: HwAccel) -> bool {
        match hwaccel {
            HwAccel::Vaapi => self.h264_vaapi,
            HwAccel::Nvenc => self.h264_nvenc,
        }
    }

    /// Prefer libfdk_aac when it is there, because it sounds better at the same bitrate.
    #[inline]
    pub const fn audio_encoder(&self) -> AudioEncoder {
        if self.libfdk_aac { AudioEncoder::LibfdkAac } else { AudioEncoder::Aac }
    }
}

/// Which part of the X11 display to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// A rectangle that is already known, so x11grab grabs only it instead of the whole screen.
    Region { position: Position, resolution: Resolution },
    /// Let x11grab ask the user to drag a rectangle; its size is known only once FFmpeg runs.
    SelectRegion,
}

/// Everything the FFmpeg command line is built from.
#[derive(Debug)]
pub struct RecordOptions<'a> {
    pub display:       &'a str,
    pub capture:       Capture,
    pub draw_mouse:    bool,
    pub frame_rate:    u32,
    pub audio_device:  Option<&'a str>,
    pub audio_encoder: AudioEncoder,
    pub duration:      Option<&'a str>,
    pub pad_to:        Option<Resolution>,
    pub crf:           u8,
    pub preset:        Option<&'a str>,
    pub hwaccel:       Option<HwAccel>,
    pub vaapi_device:  Option<&'a str>,
    pub overwrite:     bool,
    pub rtmp:          bool,
    pub output:        &'a str,
}

/// Build the whole argument list for one recording.
pub fn build_args(options: &RecordOptions) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    args.push(String::from("-hide_banner"));
    args.push(String::from(if options.overwrite { "-y" } else { "-n" }));

    if let (Some(HwAccel::Vaapi), Some(device)) = (options.hwaccel, options.vaapi_device) {
        pair(&mut args, "-vaapi_device", device);
    }

    build_video_input(&mut args, options);
    build_audio_input(&mut args, options);

    if let Some(duration) = options.duration {
        pair(&mut args, "-t", duration);
    }

    build_video_filter(&mut args, options);
    build_video_encoder(&mut args, options);
    build_audio_encoder(&mut args, options);
    build_output(&mut args, options);

    args
}

fn build_video_input(args: &mut Vec<String>, options: &RecordOptions) {
    pair(args, "-f", "x11grab");
    pair(args, "-framerate", options.frame_rate.to_string());

    if !options.draw_mouse {
        pair(args, "-draw_mouse", "0");
    }

    let input = match options.capture {
        Capture::Region {
            position,
            resolution,
        } => {
            pair(args, "-video_size", format!("{}x{}", resolution.width, resolution.height));

            format!("{}+{},{}", options.display, position.x, position.y)
        },
        Capture::SelectRegion => {
            pair(args, "-select_region", "1");

            options.display.to_string()
        },
    };

    pair(args, "-i", input);
}

fn build_audio_input(args: &mut Vec<String>, options: &RecordOptions) {
    if let Some(device) = options.audio_device {
        pair(args, "-f", "pulse");
        pair(args, "-ac", "2");
        pair(args, "-i", device);
    } else if options.rtmp {
        // RTMP services expect an audio track, so silence is generated rather than opening a sound device.
        pair(args, "-f", "lavfi");
        pair(args, "-i", "anullsrc=channel_layout=stereo:sample_rate=48000");
    }
}

fn build_video_filter(args: &mut Vec<String>, options: &RecordOptions) {
    let mut filters: Vec<String> = Vec::new();

    match options.pad_to {
        Some(resolution) => filters
            .push(format!("pad={}:{}:(ow-iw)/2:(oh-ih)/2", resolution.width, resolution.height)),
        None => {
            if options.capture == Capture::SelectRegion {
                // A dragged region can have odd dimensions, which yuv420p cannot represent.
                filters.push(String::from("pad=ceil(iw/2)*2:ceil(ih/2)*2"));
            }
        },
    }

    if options.hwaccel == Some(HwAccel::Vaapi) {
        filters.push(String::from("format=nv12"));
        filters.push(String::from("hwupload"));
    }

    if !filters.is_empty() {
        pair(args, "-vf", filters.join(","));
    }
}

fn build_video_encoder(args: &mut Vec<String>, options: &RecordOptions) {
    match options.hwaccel {
        Some(HwAccel::Vaapi) => {
            pair(args, "-c:v", "h264_vaapi");
            pair(args, "-rc_mode", "CQP");
            pair(args, "-qp", options.crf.to_string());
        },
        Some(HwAccel::Nvenc) => {
            pair(args, "-c:v", "h264_nvenc");
            pair(args, "-preset", options.preset.unwrap_or("p4"));
            pair(args, "-rc", "constqp");
            pair(args, "-qp", options.crf.to_string());
            pair(args, "-pix_fmt", "yuv420p");
        },
        None => {
            pair(args, "-c:v", "libx264");
            pair(args, "-preset", options.preset.unwrap_or("veryfast"));
            pair(args, "-pix_fmt", "yuv420p");
            pair(args, "-crf", options.crf.to_string());
        },
    }

    if options.rtmp {
        // RTMP services want a keyframe every two seconds and a bitrate that stays close to constant.
        let bitrate = rtmp_bitrate_kbps(options);

        pair(args, "-g", (options.frame_rate * 2).to_string());
        pair(args, "-maxrate", format!("{bitrate}k"));
        pair(args, "-bufsize", format!("{}k", bitrate * 2));

        if options.hwaccel.is_none() {
            pair(args, "-tune", "zerolatency");
        }
    }
}

fn build_audio_encoder(args: &mut Vec<String>, options: &RecordOptions) {
    if options.audio_device.is_none() && !options.rtmp {
        args.push(String::from("-an"));

        return;
    }

    match options.audio_encoder {
        AudioEncoder::LibfdkAac => {
            pair(args, "-c:a", "libfdk_aac");
            pair(args, "-vbr", "5");
        },
        AudioEncoder::Aac => {
            pair(args, "-c:a", "aac");
            pair(args, "-b:a", "192k");
        },
    }

    pair(args, "-ar", "48000");
}

fn build_output(args: &mut Vec<String>, options: &RecordOptions) {
    if options.rtmp {
        pair(args, "-f", "flv");
    } else if is_mp4_family(options.output) {
        // Move the index to the front of the file so that it can be played before it is fully downloaded.
        pair(args, "-movflags", "+faststart");
    }

    args.push(options.output.to_string());
}

/// Pick an RTMP bitrate in kbit/s, since a pure CRF stream can easily saturate the uplink.
fn rtmp_bitrate_kbps(options: &RecordOptions) -> u32 {
    const DEFAULT_KBPS: u32 = 4500;

    let Capture::Region {
        resolution, ..
    } = options.capture
    else {
        return DEFAULT_KBPS;
    };

    let pixels_per_second =
        resolution.width as u64 * resolution.height as u64 * options.frame_rate as u64;

    // Roughly 0.07 bit per pixel, which lands on about 4400 kbit/s for 1080p30.
    ((pixels_per_second * 7 / 100_000) as u32).clamp(1_000, 12_000)
}

/// `-movflags` only exists on the MP4 family of muxers.
fn is_mp4_family(output: &str) -> bool {
    let extension = output.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();

    matches!(extension.as_str(), "mp4" | "m4v" | "mov")
}

/// Pick the first DRM render node, which is what `-vaapi_device` expects.
pub fn find_vaapi_device() -> anyhow::Result<String> {
    let mut nodes: Vec<String> = fs::read_dir("/dev/dri")
        .context("cannot read /dev/dri; VAAPI is not available")?
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;

            name.starts_with("renderD").then(|| format!("/dev/dri/{name}"))
        })
        .collect();

    nodes.sort_unstable();

    nodes
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no DRM render node was found in /dev/dri; VAAPI is not available"))
}

#[inline]
fn pair(args: &mut Vec<String>, name: &str, value: impl AsRef<str>) {
    args.push(String::from(name));
    args.push(String::from(value.as_ref()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options<'a>(output: &'a str, rtmp: bool) -> RecordOptions<'a> {
        RecordOptions {
            display: ":0",
            capture: Capture::Region {
                position:   Position::new(0, 0),
                resolution: Resolution::new(1920, 1080),
            },
            draw_mouse: true,
            frame_rate: if rtmp { 30 } else { 60 },
            audio_device: None,
            audio_encoder: AudioEncoder::Aac,
            duration: None,
            pad_to: None,
            crf: if rtmp { 25 } else { 18 },
            preset: None,
            hwaccel: None,
            vaapi_device: None,
            overwrite: false,
            rtmp,
            output,
        }
    }

    #[test]
    fn full_screen_without_audio() {
        assert_eq!(
            [
                "-hide_banner",
                "-n",
                "-f",
                "x11grab",
                "-framerate",
                "60",
                "-video_size",
                "1920x1080",
                "-i",
                ":0+0,0",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "18",
                "-an",
                "-movflags",
                "+faststart",
                "out.mp4",
            ]
            .as_slice(),
            build_args(&options("out.mp4", false)).as_slice()
        );
    }

    #[test]
    fn window_with_audio() {
        let mut options = options("out.mp4", false);

        options.capture = Capture::Region {
            position:   Position::new(1280, 74),
            resolution: Resolution::new(1264, 702),
        };
        options.audio_device = Some("default");
        options.audio_encoder = AudioEncoder::LibfdkAac;
        options.duration = Some("30");

        assert_eq!(
            [
                "-hide_banner",
                "-n",
                "-f",
                "x11grab",
                "-framerate",
                "60",
                "-video_size",
                "1264x702",
                "-i",
                ":0+1280,74",
                "-f",
                "pulse",
                "-ac",
                "2",
                "-i",
                "default",
                "-t",
                "30",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "18",
                "-c:a",
                "libfdk_aac",
                "-vbr",
                "5",
                "-ar",
                "48000",
                "-movflags",
                "+faststart",
                "out.mp4",
            ]
            .as_slice(),
            build_args(&options).as_slice()
        );
    }

    #[test]
    fn rtmp_without_audio_streams_silence() {
        assert_eq!(
            [
                "-hide_banner",
                "-n",
                "-f",
                "x11grab",
                "-framerate",
                "30",
                "-video_size",
                "1920x1080",
                "-i",
                ":0+0,0",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=channel_layout=stereo:sample_rate=48000",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "25",
                "-g",
                "60",
                "-maxrate",
                "4354k",
                "-bufsize",
                "8708k",
                "-tune",
                "zerolatency",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-ar",
                "48000",
                "-f",
                "flv",
                "rtmp://example.org/live",
            ]
            .as_slice(),
            build_args(&options("rtmp://example.org/live", true)).as_slice()
        );
    }

    #[test]
    fn normalize_adds_a_pad_filter() {
        let mut options = options("out.mp4", false);

        options.pad_to = Some(Resolution::new(3840, 2160));

        assert!(build_args(&options).windows(2).any(|window| {
            window[0] == "-vf" && window[1] == "pad=3840:2160:(ow-iw)/2:(oh-ih)/2"
        }));
    }

    #[test]
    fn vaapi_uploads_frames_to_the_gpu() {
        let mut options = options("out.mp4", false);

        options.hwaccel = Some(HwAccel::Vaapi);
        options.vaapi_device = Some("/dev/dri/renderD128");

        let args = build_args(&options);

        assert!(args.windows(2).any(|w| w[0] == "-vf" && w[1] == "format=nv12,hwupload"));
        assert!(args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "h264_vaapi"));
        assert!(args.windows(2).any(|w| w[0] == "-vaapi_device" && w[1] == "/dev/dri/renderD128"));
        assert!(!args.iter().any(|arg| arg == "-pix_fmt"));
    }
}
