use std::{
    io::{self, ErrorKind},
    process::{self, Stdio},
    str::from_utf8_unchecked,
};

use anyhow::{anyhow, Context};
use execute::{command, Execute};

use crate::{Position, Resolution};

#[derive(Debug)]
pub struct WindowInfo {
    pub screen:     Resolution,
    pub resolution: Resolution,
    pub position:   Position,
}

impl WindowInfo {
    pub fn new() -> anyhow::Result<WindowInfo> {
        let screen_resolution = Resolution::get_screen_resolution()?;

        let mut command = command!("xwininfo");

        command.stdout(Stdio::piped());

        let output = command.execute_output().with_context(|| anyhow!("xwininfo"))?;

        if output.status.code().is_none() {
            process::exit(1);
        }

        let win_info = output.stdout;

        let mut command1 = command!("grep 'Width:'");
        let mut command2 = command!("cut -d ':' -f 2");
        let mut command3 = command!("tr -d ' '");

        command3.stdout(Stdio::piped());

        let output = command1
            .execute_multiple_input_output(win_info.as_slice(), &mut [&mut command2, &mut command3])
            .with_context(|| {
                anyhow!("grep 'Width:' from {:?}", String::from_utf8_lossy(win_info.as_slice()))
            })?;

        let win_width: i32 = unsafe { from_utf8_unchecked(output.stdout.as_slice()) }
            .trim_end()
            .parse()
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;

        let mut command1 = command!("grep 'Height:'");

        let output = command1
            .execute_multiple_input_output(win_info.as_slice(), &mut [&mut command2, &mut command3])
            .with_context(|| {
                anyhow!("grep 'Height:' from {:?}", String::from_utf8_lossy(win_info.as_slice()))
            })?;

        let win_height: i32 =
            unsafe { from_utf8_unchecked(output.stdout.as_slice()) }.trim_end().parse().unwrap();

        let mut command1 = command!("grep 'Absolute upper-left X'");

        let output = command1
            .execute_multiple_input_output(win_info.as_slice(), &mut [&mut command2, &mut command3])
            .with_context(|| {
                anyhow!(
                    "grep 'Absolute upper-left X' from {:?}",
                    String::from_utf8_lossy(win_info.as_slice())
                )
            })?;

        let win_ux: i32 = unsafe { from_utf8_unchecked(output.stdout.as_slice()) }
            .trim_end()
            .parse()
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;

        let mut command1 = command!("grep 'Absolute upper-left Y'");

        let output = command1.execute_multiple_input_output(win_info.as_slice(), &mut [
            &mut command2,
            &mut command3,
        ])?;

        let win_uy: i32 =
            unsafe { from_utf8_unchecked(output.stdout.as_slice()) }.trim_end().parse().unwrap();

        let width = if win_width + win_ux > screen_resolution.width {
            screen_resolution.width - win_ux
        } else {
            win_width
        };

        let height = if win_height + win_uy > screen_resolution.height {
            screen_resolution.height - win_uy
        } else {
            win_height
        };

        Ok(WindowInfo {
            screen:     screen_resolution,
            resolution: Resolution::new(width, height),
            position:   Position::new(win_ux, win_uy),
        })
    }
}
