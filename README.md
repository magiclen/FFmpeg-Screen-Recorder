FFmpeg Screen Recorder
====================

[![CI](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml)

This program is a gadget which helps you use **FFmpeg** to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol.

Only **X11** is supported for now. In a Wayland session, `x11grab` can merely capture an empty XWayland screen, so this program refuses to run instead of writing a black video.

## Setup

All you need is **FFmpeg** 8 or newer. You can get one [here](https://ffmpeg.org/). It has to be compiled with the **libxcb** and **libx264** libraries.

**libfdk-aac** is optional. It is used for the audio track when the FFmpeg being used has it, and the built-in AAC encoder is used otherwise.

`xrandr` is used to detect the monitors, `xwininfo` is used by `--window`, and `pactl` is used by `--list-audio-devices`.

## Help

```
EXAMPLES:
ffmpeg-screen-recorder                         # Record every monitor without audio into the current working directory
ffmpeg-screen-recorder -w                      # Select a window with the mouse and record it
ffmpeg-screen-recorder --select-region         # Drag a rectangle with the mouse and record it
ffmpeg-screen-recorder -m HDMI-0               # Record a single monitor
ffmpeg-screen-recorder -a                      # Record with the system audio
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
          Record your screen with audio which could be internal or external. It depends on your computer environment

      --audio-device <DEVICE>
          Record from this PulseAudio source, which also implies --with-audio [default: default]

      --list-audio-devices
          List the PulseAudio sources that --audio-device accepts, then exit

      --normalize
          Pad the video size with black borders to the fixed ratio of 16:9

      --no-mouse
          Do not record the mouse pointer

      --fps <FPS>
          Set the frame rate [default: 60, or 30 when streaming]

      --crf <CRF>
          Set the quality, where a smaller number means a bigger file [default: 18, or 25 when streaming]

      --preset <PRESET>
          Set the encoder preset, such as ultrafast or slow [default: veryfast]

      --hwaccel <BACKEND>
          Let the GPU encode the video instead of the CPU

          Possible values:
          - vaapi: Intel and AMD GPUs, through `h264_vaapi`
          - nvenc: NVIDIA GPUs, through `h264_nvenc`

  -t, --duration <DURATION>
          Stop after this long, as seconds or as HH:MM:SS

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

Press `Ctrl-C` to stop a recording. FFmpeg is given the chance to finish writing the file before this program exits.

## License

[MIT](LICENSE)
