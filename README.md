FFmpeg Screen Recorder
====================

This program a gadget which helps you use **FFmpeg** to record your screen on Linux. The video record can be saved as a
file, or be streamed via RTMP protocol.

## Seting Up

All you need is **FFmpeg**. You can get one [here](https://github.com/magiclen/FFmpeg-For-MagicLen-Applications/releases). It has to be compiled with **libxcb**, **libfdk-aac** and **libx264** libraries.

## Help

```
USAGE:
    ffmpeg-screen-recorder [FLAGS] [OPTIONS]

FLAGS:
    -a, --with-audio      Records your screen with audio which could be internal or external. It depends on your
                          computer environment.
    -h, --help            Prints help information
    -n, --no-normalize    Does not pad the video size with black borders to the fixed ratio of 16:9.
    -V, --version         Prints version information
    -w, --window          Selects a window to record.

OPTIONS:
    -f, --ffmpeg-path <FFMPEG_PATH>    Specifies the path of your FFmpeg executable binary file. [default: ffmpeg]
    -o, --output <FILE/RTMP_URL>       Assigns a destination of your video. It should be a file path or a RTMP url.
```

## License

[MIT](LICENSE)