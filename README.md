FFmpeg Screen Recorder
====================

[![CI](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml)

This program is a gadget which helps you use **FFmpeg** to record your screen on Linux. The video record can be saved as a file, or be streamed via RTMP protocol.

## Setup

All you need is **FFmpeg**. You can get one [here](https://ffmpeg.org/). It has to be compiled with **libxcb**, **libfdk-aac** and **libx264** libraries.

## Help

```
EXAMPLES:
ffmpeg-screen-recorder                    # Record the full screen without audio and output into the current working directory
ffmpeg-screen-recorder -w                 # Select a window and record it without audio and output into the current working directory
ffmpeg-screen-recorder -a                 # Record the full screen with the system audio and output into the current working directory
ffmpeg-screen-recorder -o /path/to/file   # Record the full screen without audio and output to /path/to/file
ffmpeg-screen-recorder -o rtmp://xxx      # Record the full screen without audio and output to rtmp://xxx

Usage: ffmpeg-screen-recorder [OPTIONS]

Options:
  -w, --window                     Select a window to record
  -a, --with-audio                 Record your screen with audio which could be internal or external. It depends on your computer environment
  -n, --no-normalize               Do not pad the video size with black borders to the fixed ratio of 16:9 [aliases: nn]
  -o, --output <FILE/RTMP_URL>     Assign a destination of your video. It should be a file path or a RTMP url [default: CWD/<time>.mp4]
  -f, --ffmpeg-path <FFMPEG_PATH>  Specify the path of your FFmpeg executable binary file [default: ffmpeg]
  -h, --help                       Print help
  -V, --version                    Print version
```

## License

[MIT](LICENSE)