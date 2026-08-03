use std::process::{Command, Stdio};

use anyhow::{Context, anyhow};

use crate::{position::Position, resolution::Resolution};

/// One active monitor of the X11 display, as reported by `xrandr --listmonitors`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    pub name:       String,
    pub position:   Position,
    pub resolution: Resolution,
    pub primary:    bool,
}

impl Monitor {
    /// Ask xrandr for every active monitor of the current display.
    pub fn list() -> anyhow::Result<Vec<Monitor>> {
        let output = Command::new("xrandr")
            .arg("--listmonitors")
            .stdout(Stdio::piped())
            .output()
            .context("cannot execute `xrandr`; it is needed to detect the screen size")?;

        if !output.status.success() {
            return Err(anyhow!(
                "`xrandr --listmonitors` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }

        let text = String::from_utf8(output.stdout)
            .context("`xrandr --listmonitors` did not output UTF-8 text")?;

        let monitors = parse_monitors(&text);

        if monitors.is_empty() {
            return Err(anyhow!(
                "`xrandr --listmonitors` reported no monitor:\n{}",
                text.trim_end()
            ));
        }

        Ok(monitors)
    }
}

/// The area x11grab captures when no single monitor is picked, which is the box around every monitor.
pub fn bounding_box(monitors: &[Monitor]) -> Resolution {
    let width = monitors.iter().map(|m| m.position.x + m.resolution.width).max().unwrap_or(0);
    let height = monitors.iter().map(|m| m.position.y + m.resolution.height).max().unwrap_or(0);

    Resolution::new(width, height)
}

/// Parse `xrandr --listmonitors`, whose entries look like ` 0: +*HDMI-0 3840/597x2160/336+0+0  HDMI-0`.
fn parse_monitors(text: &str) -> Vec<Monitor> {
    let mut monitors = Vec::new();

    for line in text.lines() {
        // The header line `Monitors: 1` is skipped here, because its first field is not an index.
        let Some((index, rest)) = line.trim().split_once(':') else {
            continue;
        };

        if index.trim().parse::<u32>().is_err() {
            continue;
        }

        let mut tokens = rest.split_whitespace();

        let (Some(name), Some(geometry)) = (tokens.next(), tokens.next()) else {
            continue;
        };

        let Some((resolution, position)) = parse_geometry(geometry) else {
            continue;
        };

        monitors.push(Monitor {
            // xrandr prefixes the name with `+` when the monitor is automatic and `*` when it is primary.
            name: name.trim_start_matches(['+', '*']).to_string(),
            position,
            resolution,
            primary: name.contains('*'),
        });
    }

    monitors
}

/// Split `3840/597x2160/336+0+0` into its size and its offset; the millimetre parts are optional.
fn parse_geometry(geometry: &str) -> Option<(Resolution, Position)> {
    let (width_part, rest) = geometry.split_once('x')?;

    let width = width_part.split('/').next()?.trim().parse().ok()?;

    let mut parts = rest.split('+');

    let height = parts.next()?.split('/').next()?.trim().parse().ok()?;
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;

    Some((Resolution::new(width, height), Position::new(x, y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Monitors: 2\n 0: +*HDMI-0 3840/597x2160/336+0+0  HDMI-0\n 1: +DP-2 \
                          1920/527x1080/296+3840+120  DP-2\n";

    #[test]
    fn parse_listmonitors_output() {
        let monitors = parse_monitors(SAMPLE);

        assert_eq!(2, monitors.len());

        assert_eq!(
            Monitor {
                name:       String::from("HDMI-0"),
                position:   Position::new(0, 0),
                resolution: Resolution::new(3840, 2160),
                primary:    true,
            },
            monitors[0]
        );

        assert_eq!(
            Monitor {
                name:       String::from("DP-2"),
                position:   Position::new(3840, 120),
                resolution: Resolution::new(1920, 1080),
                primary:    false,
            },
            monitors[1]
        );
    }

    #[test]
    fn bounding_box_covers_every_monitor() {
        assert_eq!(Resolution::new(5760, 2160), bounding_box(&parse_monitors(SAMPLE)));
    }

    #[test]
    fn parse_geometry_without_millimetres() {
        assert_eq!(
            Some((Resolution::new(1920, 1080), Position::new(10, 20))),
            parse_geometry("1920x1080+10+20")
        );
    }
}
