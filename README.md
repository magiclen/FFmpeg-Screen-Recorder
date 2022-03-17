FFmpeg Screen Recorder
====================

[![CI](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/FFmpeg-Screen-Recorder/actions/workflows/ci.yml)

This program is a gadget which helps you use **FFmpeg** to record your screen on Linux. The video record can be saved as a
file, or be streamed via RTMP protocol.

## Seting Up

All you need is **FFmpeg**. You can get one [here](https://github.com/magiclen/FFmpeg-For-MagicLen-Applications/releases). It has to be compiled with **libxcb**, **libfdk-aac** and **libx264** libraries.

## Help

```
USAGE:
    ffmpeg-screen-recorder [OPTIONS]

OPTIONS:
    -a, --with-audio                   Record your screen with audio which could be internal or external. It depends on your computer environment.
    -f, --ffmpeg-path <FFMPEG_PATH>    Specify the path of your FFmpeg executable binary file. [default: ffmpeg]
    -h, --help                         Print help information
    -n, --no-normalize                 Do not pad the video size with black borders to the fixed ratio of 16:9.
    -o, --output <FILE/RTMP_URL>       Assign a destination of your video. It should be a file path or a RTMP url.
    -V, --version                      Print version information
    -w, --window                       Select a window to record.
```

## License

[MIT](LICENSE)