use std::{fs, process::Command, str::FromStr};

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

/// The PulseAudio sources to record alongside the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCapture<'a> {
    None,
    Device(&'a str),
    Mix { system: &'a str, microphone: &'a str },
}

/// A libx264 speed and compression preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwarePreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
    Placebo,
}

impl SoftwarePreset {
    #[inline]
    const fn as_str(self) -> &'static str {
        match self {
            SoftwarePreset::Ultrafast => "ultrafast",
            SoftwarePreset::Superfast => "superfast",
            SoftwarePreset::Veryfast => "veryfast",
            SoftwarePreset::Faster => "faster",
            SoftwarePreset::Fast => "fast",
            SoftwarePreset::Medium => "medium",
            SoftwarePreset::Slow => "slow",
            SoftwarePreset::Slower => "slower",
            SoftwarePreset::Veryslow => "veryslow",
            SoftwarePreset::Placebo => "placebo",
        }
    }
}

impl FromStr for SoftwarePreset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ultrafast" => Ok(SoftwarePreset::Ultrafast),
            "superfast" => Ok(SoftwarePreset::Superfast),
            "veryfast" => Ok(SoftwarePreset::Veryfast),
            "faster" => Ok(SoftwarePreset::Faster),
            "fast" => Ok(SoftwarePreset::Fast),
            "medium" => Ok(SoftwarePreset::Medium),
            "slow" => Ok(SoftwarePreset::Slow),
            "slower" => Ok(SoftwarePreset::Slower),
            "veryslow" => Ok(SoftwarePreset::Veryslow),
            "placebo" => Ok(SoftwarePreset::Placebo),
            _ => Err(format!(
                "invalid libx264 preset {value:?}; expected ultrafast, superfast, veryfast, \
                 faster, fast, medium, slow, slower, veryslow, or placebo"
            )),
        }
    }
}

/// An NVIDIA NVENC speed and compression preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvencPreset {
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    Fast,
    Medium,
    Slow,
}

impl NvencPreset {
    #[inline]
    const fn as_str(self) -> &'static str {
        match self {
            NvencPreset::P1 => "p1",
            NvencPreset::P2 => "p2",
            NvencPreset::P3 => "p3",
            NvencPreset::P4 => "p4",
            NvencPreset::P5 => "p5",
            NvencPreset::P6 => "p6",
            NvencPreset::P7 => "p7",
            NvencPreset::Fast => "fast",
            NvencPreset::Medium => "medium",
            NvencPreset::Slow => "slow",
        }
    }
}

impl FromStr for NvencPreset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "p1" => Ok(NvencPreset::P1),
            "p2" => Ok(NvencPreset::P2),
            "p3" => Ok(NvencPreset::P3),
            "p4" => Ok(NvencPreset::P4),
            "p5" => Ok(NvencPreset::P5),
            "p6" => Ok(NvencPreset::P6),
            "p7" => Ok(NvencPreset::P7),
            "fast" => Ok(NvencPreset::Fast),
            "medium" => Ok(NvencPreset::Medium),
            "slow" => Ok(NvencPreset::Slow),
            _ => Err(format!(
                "invalid NVENC preset {value:?}; expected p1, p2, p3, p4, p5, p6, p7, fast, \
                 medium, or slow"
            )),
        }
    }
}

/// The selected H.264 encoder and its backend-specific options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoEncoder<'a> {
    Libx264 { crf: u8, preset: SoftwarePreset },
    Vaapi { device: &'a str, qp: u8 },
    Nvenc { gpu: Option<u32>, qp: u8, preset: NvencPreset },
}

/// Which recording components the FFmpeg being used provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub libx264:    bool,
    pub aac:        bool,
    pub libfdk_aac: bool,
    pub h264_vaapi: bool,
    pub h264_nvenc: bool,
    pub x11grab:    bool,
    pub pulse:      bool,
    pub lavfi:      bool,
    pub amix:       bool,
    pub anullsrc:   bool,
    pub flv:        bool,
}

impl Capabilities {
    /// Ask FFmpeg which recording components this program can use.
    pub fn probe(ffmpeg_path: &str) -> anyhow::Result<Capabilities> {
        let encoders = ffmpeg_listing(ffmpeg_path, "-encoders", "encoders")?;
        let devices = ffmpeg_listing(ffmpeg_path, "-devices", "input devices")?;
        let filters = ffmpeg_listing(ffmpeg_path, "-filters", "filters")?;
        let muxers = ffmpeg_listing(ffmpeg_path, "-muxers", "muxers")?;

        Ok(Capabilities {
            libx264:    listing_has(&encoders, "libx264"),
            aac:        listing_has(&encoders, "aac"),
            libfdk_aac: listing_has(&encoders, "libfdk_aac"),
            h264_vaapi: listing_has(&encoders, "h264_vaapi"),
            h264_nvenc: listing_has(&encoders, "h264_nvenc"),
            x11grab:    listing_has(&devices, "x11grab"),
            pulse:      listing_has(&devices, "pulse"),
            lavfi:      listing_has(&devices, "lavfi"),
            amix:       listing_has(&filters, "amix"),
            anullsrc:   listing_has(&filters, "anullsrc"),
            flv:        listing_has(&muxers, "flv"),
        })
    }

    pub fn validate(
        &self,
        ffmpeg_path: &str,
        hwaccel: Option<HwAccel>,
        audio_capture: AudioCapture,
        rtmp: bool,
    ) -> anyhow::Result<()> {
        require_capability(self.x11grab, ffmpeg_path, "the x11grab input device")?;

        match hwaccel {
            Some(hwaccel) => require_capability(
                self.has(hwaccel),
                ffmpeg_path,
                &format!("the {} encoder", hwaccel.encoder()),
            )?,
            None => require_capability(self.libx264, ffmpeg_path, "the libx264 encoder")?,
        }

        if audio_capture != AudioCapture::None {
            require_capability(self.pulse, ffmpeg_path, "the PulseAudio input device")?;
        }
        if matches!(audio_capture, AudioCapture::Mix { .. }) {
            require_capability(self.amix, ffmpeg_path, "the amix audio filter")?;
        }
        if rtmp && audio_capture == AudioCapture::None {
            require_capability(self.lavfi, ffmpeg_path, "the lavfi input device")?;
            require_capability(self.anullsrc, ffmpeg_path, "the anullsrc audio filter")?;
        }
        if rtmp {
            require_capability(self.flv, ffmpeg_path, "the FLV muxer")?;
        }
        if audio_capture != AudioCapture::None || rtmp {
            require_capability(
                self.libfdk_aac || self.aac,
                ffmpeg_path,
                "the libfdk_aac or native AAC encoder",
            )?;
        }

        Ok(())
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

fn ffmpeg_listing(ffmpeg_path: &str, option: &str, description: &str) -> anyhow::Result<String> {
    let output = Command::new(ffmpeg_path)
        .args(["-hide_banner", option])
        .output()
        .with_context(|| anyhow!("cannot execute {ffmpeg_path:?}"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "{ffmpeg_path:?} cannot list its {description}; FFmpeg 8 or newer is expected"
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn listing_has(listing: &str, name: &str) -> bool {
    listing.lines().any(|line| line.split_whitespace().any(|field| field == name))
}

fn require_capability(available: bool, ffmpeg_path: &str, capability: &str) -> anyhow::Result<()> {
    if available { Ok(()) } else { Err(anyhow!("{ffmpeg_path:?} does not provide {capability}")) }
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
    pub audio_capture: AudioCapture<'a>,
    pub audio_encoder: AudioEncoder,
    pub duration:      Option<&'a str>,
    pub pad_to:        Option<Resolution>,
    pub video_encoder: VideoEncoder<'a>,
    pub overwrite:     bool,
    pub rtmp:          bool,
    pub output:        &'a str,
}

/// Build the whole argument list for one recording.
pub fn build_args(options: &RecordOptions) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    args.push(String::from("-hide_banner"));
    args.push(String::from(if options.overwrite { "-y" } else { "-n" }));

    if let VideoEncoder::Vaapi {
        device, ..
    } = options.video_encoder
    {
        pair(&mut args, "-vaapi_device", device);
    }

    build_video_input(&mut args, options);
    build_audio_input(&mut args, options);

    if let Some(duration) = options.duration {
        pair(&mut args, "-t", duration);
    }

    build_video_filter(&mut args, options);
    build_audio_filter(&mut args, options);
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
    match options.audio_capture {
        AudioCapture::Device(device) => build_pulse_input(args, device),
        AudioCapture::Mix {
            system,
            microphone,
        } => {
            build_pulse_input(args, system);
            build_pulse_input(args, microphone);
        },
        AudioCapture::None => {
            if options.rtmp {
                // RTMP services expect an audio track, so silence is generated rather than opening a sound device.
                pair(args, "-f", "lavfi");
                pair(args, "-i", "anullsrc=channel_layout=stereo:sample_rate=48000");
            }
        },
    }
}

fn build_pulse_input(args: &mut Vec<String>, device: &str) {
    pair(args, "-f", "pulse");
    pair(args, "-ac", "2");
    pair(args, "-i", device);
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

    if matches!(options.video_encoder, VideoEncoder::Vaapi { .. }) {
        filters.push(String::from("format=nv12"));
        filters.push(String::from("hwupload"));
    }

    if !filters.is_empty() {
        pair(args, "-vf", filters.join(","));
    }
}

fn build_audio_filter(args: &mut Vec<String>, options: &RecordOptions) {
    if matches!(options.audio_capture, AudioCapture::Mix { .. }) {
        pair(
            args,
            "-filter_complex",
            "[1:a][2:a]amix=inputs=2:duration=longest:dropout_transition=0[audio]",
        );
        pair(args, "-map", "0:v");
        pair(args, "-map", "[audio]");
    }
}

fn build_video_encoder(args: &mut Vec<String>, options: &RecordOptions) {
    match options.video_encoder {
        VideoEncoder::Vaapi {
            qp, ..
        } => {
            pair(args, "-c:v", "h264_vaapi");

            if options.rtmp {
                let bitrate = rtmp_bitrate_kbps(options);

                pair(args, "-rc_mode", "VBR");
                pair(args, "-b:v", format!("{bitrate}k"));
            } else {
                pair(args, "-rc_mode", "CQP");
                pair(args, "-qp", qp.to_string());
            }
        },
        VideoEncoder::Nvenc {
            gpu,
            qp,
            preset,
        } => {
            pair(args, "-c:v", "h264_nvenc");
            pair(args, "-preset", preset.as_str());

            if let Some(gpu) = gpu {
                pair(args, "-gpu", gpu.to_string());
            }

            if options.rtmp {
                pair(args, "-tune", "ull");
                pair(args, "-rc", "vbr");
                pair(args, "-cq", qp.to_string());
                pair(args, "-b:v", "0");
            } else {
                pair(args, "-rc", "constqp");
                pair(args, "-qp", qp.to_string());
            }

            pair(args, "-pix_fmt", "yuv420p");
        },
        VideoEncoder::Libx264 {
            crf,
            preset,
        } => {
            pair(args, "-c:v", "libx264");
            pair(args, "-preset", preset.as_str());
            pair(args, "-pix_fmt", "yuv420p");
            pair(args, "-crf", crf.to_string());
        },
    }

    if options.rtmp {
        // RTMP services want a keyframe every two seconds and a bitrate that stays close to constant.
        let bitrate = rtmp_bitrate_kbps(options);

        pair(args, "-g", (options.frame_rate * 2).to_string());
        pair(args, "-maxrate", format!("{bitrate}k"));
        pair(args, "-bufsize", format!("{}k", bitrate * 2));

        if matches!(options.video_encoder, VideoEncoder::Libx264 { .. }) {
            pair(args, "-tune", "zerolatency");
        }
    }
}

fn build_audio_encoder(args: &mut Vec<String>, options: &RecordOptions) {
    if options.audio_capture == AudioCapture::None && !options.rtmp {
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

    fn value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
        args.windows(2).find(|pair| pair[0] == name).map(|pair| pair[1].as_str())
    }

    fn options<'a>(output: &'a str, rtmp: bool) -> RecordOptions<'a> {
        RecordOptions {
            display: ":0",
            capture: Capture::Region {
                position:   Position::new(0, 0),
                resolution: Resolution::new(1920, 1080),
            },
            draw_mouse: true,
            frame_rate: if rtmp { 30 } else { 60 },
            audio_capture: AudioCapture::None,
            audio_encoder: AudioEncoder::Aac,
            duration: None,
            pad_to: None,
            video_encoder: VideoEncoder::Libx264 {
                crf:    if rtmp { 25 } else { 18 },
                preset: SoftwarePreset::Veryfast,
            },
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
        options.audio_capture = AudioCapture::Device("default");
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
    fn system_audio_and_microphone_are_mixed() {
        let mut options = options("out.mp4", false);

        options.audio_capture = AudioCapture::Mix {
            system:     "@DEFAULT_MONITOR@",
            microphone: "@DEFAULT_SOURCE@",
        };

        let args = build_args(&options);
        let devices: Vec<&str> =
            args.windows(2).filter(|pair| pair[0] == "-i").map(|pair| pair[1].as_str()).collect();
        let mappings: Vec<&str> =
            args.windows(2).filter(|pair| pair[0] == "-map").map(|pair| pair[1].as_str()).collect();

        assert_eq!([":0+0,0", "@DEFAULT_MONITOR@", "@DEFAULT_SOURCE@"], devices.as_slice());
        assert_eq!(["0:v", "[audio]"], mappings.as_slice());
        assert_eq!(
            Some("[1:a][2:a]amix=inputs=2:duration=longest:dropout_transition=0[audio]"),
            value(&args, "-filter_complex")
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

        options.video_encoder = VideoEncoder::Vaapi {
            device: "/dev/dri/renderD128", qp: 18
        };

        let args = build_args(&options);

        assert!(args.windows(2).any(|w| w[0] == "-vf" && w[1] == "format=nv12,hwupload"));
        assert!(args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "h264_vaapi"));
        assert!(args.windows(2).any(|w| w[0] == "-vaapi_device" && w[1] == "/dev/dri/renderD128"));
        assert!(!args.iter().any(|arg| arg == "-pix_fmt"));
    }

    #[test]
    fn nvenc_stream_uses_capped_vbr() {
        let mut options = options("rtmp://example.org/live", true);

        options.video_encoder =
            VideoEncoder::Nvenc {
                gpu: Some(2), qp: 25, preset: NvencPreset::P6
            };

        let args = build_args(&options);

        assert_eq!(Some("h264_nvenc"), value(&args, "-c:v"));
        assert_eq!(Some("p6"), value(&args, "-preset"));
        assert_eq!(Some("2"), value(&args, "-gpu"));
        assert_eq!(Some("vbr"), value(&args, "-rc"));
        assert_eq!(Some("25"), value(&args, "-cq"));
        assert_eq!(Some("0"), value(&args, "-b:v"));
        assert_eq!(Some("4354k"), value(&args, "-maxrate"));
        assert_eq!(None, value(&args, "-qp"));
    }

    #[test]
    fn vaapi_stream_uses_targeted_vbr() {
        let mut options = options("rtmp://example.org/live", true);

        options.video_encoder = VideoEncoder::Vaapi {
            device: "/dev/dri/renderD129", qp: 25
        };

        let args = build_args(&options);

        assert_eq!(Some("h264_vaapi"), value(&args, "-c:v"));
        assert_eq!(Some("/dev/dri/renderD129"), value(&args, "-vaapi_device"));
        assert_eq!(Some("VBR"), value(&args, "-rc_mode"));
        assert_eq!(Some("4354k"), value(&args, "-b:v"));
        assert_eq!(Some("4354k"), value(&args, "-maxrate"));
        assert_eq!(None, value(&args, "-qp"));
    }
}
