FFmpeg Screen Recorder
====================

[![CI](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml)

This program is a gadget which helps you use **FFmpeg** to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol.

Only **X11** is supported for now. In a Wayland session, `x11grab` can merely capture an empty XWayland screen, so this program refuses to run instead of writing a black video.

## Setup

All you need is **FFmpeg** 8 or newer. You can get one [here](https://ffmpeg.org/). It has to be compiled with **libxcb** and the selected H.264 encoder: **libx264** by default, **h264_vaapi** for VAAPI, or **h264_nvenc** for NVENC.

**libfdk-aac** is optional. It is used for the audio track when the FFmpeg being used has it, and the built-in AAC encoder is used otherwise.

`xrandr` is used to detect the monitors, `xwininfo` is used by `--window`, and `pactl` is used by `--list-audio-devices`.

`--with-audio` records the default system output, while `--microphone` records the default input. Use both to mix them into one audio track, or use `--audio-device` to record a specific PulseAudio source.

`--quality` uses CRF with libx264 and QP/CQ with VAAPI or NVENC. Smaller values preserve more detail. The default libx264 preset is `veryfast`; accepted values are `ultrafast`, `superfast`, `veryfast`, `faster`, `fast`, `medium`, `slow`, `slower`, `veryslow`, and `placebo`. The default NVENC preset is `p4`; accepted values are `p1` through `p7`, `fast`, `medium`, and `slow`. VAAPI does not use presets.

`--normalize` pads to the smallest common 16:9 resolution that can contain the captured area, from 426x240 through 15360x8640 (16K).

## Help

```
EXAMPLES:
ffmpeg-screen-recorder                         # Record every monitor without audio into the current working directory
ffmpeg-screen-recorder -w                      # Select a window with the mouse and record it
ffmpeg-screen-recorder --select-region         # Drag a rectangle with the mouse and record it
ffmpeg-screen-recorder -m HDMI-0               # Record a single monitor
ffmpeg-screen-recorder -a                      # Record with the system audio
ffmpeg-screen-recorder --microphone            # Record with the default microphone
ffmpeg-screen-recorder -a --microphone         # Mix the system audio and default microphone
ffmpeg-screen-recorder -t 30 -o /path/to/file  # Record for 30 seconds and output to /path/to/file
ffmpeg-screen-recorder -o rtmp://xxx           # Stream to an RTMP url
ffmpeg-screen-recorder --hwaccel vaapi         # Let the GPU encode instead of the CPU

Usage: ffmpeg-screen-recorder [OPTIONS]

Options:
  -w, --window
          Select a window to record

      --select-region
          Drag a rectangle with your mouse and record only that area

  -m, --monitor <MONITOR>
          Record one monitor, named as in `xrandr --listmonitors` [default: every monitor]

  -a, --with-audio
          Record the sound playing through the default output device

      --microphone
          Record from the default microphone; combine it with --with-audio to mix both

      --audio-device <DEVICE>
          Record only from this PulseAudio source

      --list-audio-devices
          List the PulseAudio sources that --audio-device accepts, then exit

      --normalize
          Pad the capture to the smallest fitting common 16:9 resolution, up to 16K

      --no-mouse
          Do not record the mouse pointer

      --fps <FPS>
          Set the frame rate [default: 60, or 30 when streaming]

      --quality <QUALITY>
          Set recording quality; a smaller number means a bigger file [default: 18, or 25 when streaming]

      --preset <PRESET>
          Choose a libx264 or NVENC preset; VAAPI does not support presets

      --hwaccel <BACKEND>
          Let the GPU encode the video instead of the CPU

          Possible values:
          - vaapi: Intel and AMD GPUs, through `h264_vaapi`
          - nvenc: NVIDIA GPUs, through `h264_nvenc`

      --vaapi-device <PATH>
          Select a DRM render node for --hwaccel vaapi [default: first render node]

      --nvenc-gpu <INDEX>
          Select a GPU index for --hwaccel nvenc [default: first capable GPU]

  -t, --duration <DURATION>
          Stop after this long, as seconds, MM:SS, or HH:MM:SS

  -y, --overwrite
          Overwrite the output file if it already exists

  -d, --display <DISPLAY>
          Record this X display [default: $DISPLAY]

  -o, --output <FILE/RTMP_URL>
          Assign a destination of your video. It should be a file path or a RTMP url [default: CWD/<time>.mp4]

  -f, --ffmpeg-path <FFMPEG_PATH>
          Specify the path of your FFmpeg executable binary file
          
          [default: ffmpeg]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Press `Ctrl-C`, or send `SIGTERM`, to stop a recording. FFmpeg is given the chance to finish writing the file before this program exits.

## License

[MIT](LICENSE)
