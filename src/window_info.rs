use std::process::{Command, Stdio};

use anyhow::{Context, anyhow};

use crate::{position::Position, resolution::Resolution};

/// The geometry of a window the user picked with the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowInfo {
    pub resolution: Resolution,
    pub position:   Position,
}

impl WindowInfo {
    /// Let the user click a window and report its geometry.
    pub fn select(display: &str) -> anyhow::Result<WindowInfo> {
        let output = Command::new("xwininfo")
            .args(["-display", display])
            .stdout(Stdio::piped())
            .output()
            .context("cannot execute `xwininfo`; it is needed by --window")?;

        if !output.status.success() {
            return Err(anyhow!("`xwininfo` failed; no window was selected"));
        }

        let text =
            String::from_utf8(output.stdout).context("`xwininfo` did not output UTF-8 text")?;

        parse_window_info(&text)
    }

    /// Clip the window to the screen, because x11grab rejects negative or out-of-screen regions.
    pub fn clip_to(self, screen: Resolution) -> anyhow::Result<WindowInfo> {
        let x = self.position.x.max(0);
        let y = self.position.y.max(0);

        let width = (self.position.x + self.resolution.width).min(screen.width) - x;
        let height = (self.position.y + self.resolution.height).min(screen.height) - y;

        if width <= 0 || height <= 0 {
            return Err(anyhow!(
                "the selected window lies outside of the {}x{} screen",
                screen.width,
                screen.height
            ));
        }

        Ok(WindowInfo {
            resolution: Resolution::new(width, height),
            position:   Position::new(x, y),
        })
    }
}

/// Read the geometry out of an `xwininfo` report.
fn parse_window_info(text: &str) -> anyhow::Result<WindowInfo> {
    let width = field(text, "Width")?;
    let height = field(text, "Height")?;
    let x = field(text, "Absolute upper-left X")?;
    let y = field(text, "Absolute upper-left Y")?;

    Ok(WindowInfo {
        resolution: Resolution::new(width, height), position: Position::new(x, y)
    })
}

/// Read one `Name: value` line out of an `xwininfo` report.
fn field(text: &str, name: &str) -> anyhow::Result<i32> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(name)?.strip_prefix(':')?.trim().parse().ok())
        .ok_or_else(|| anyhow!("cannot read {name:?} from the output of `xwininfo`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
xwininfo: Window id: 0x4400007 "Terminal"

  Absolute upper-left X:  1280
  Absolute upper-left Y:  74
  Relative upper-left X:  0
  Relative upper-left Y:  0
  Width: 1265
  Height: 703
  Depth: 24
  Border width: 0
  Corners:  +1280+74  -1295+74  -1295-1383  +1280-1383
  -geometry 1265x703+1280+74
"#;

    #[test]
    fn parse_xwininfo_output() {
        assert_eq!(
            WindowInfo {
                resolution: Resolution::new(1265, 703),
                position:   Position::new(1280, 74),
            },
            parse_window_info(SAMPLE).unwrap()
        );
    }

    #[test]
    fn clip_keeps_a_fully_visible_window() {
        let window = WindowInfo {
            resolution: Resolution::new(800, 600),
            position:   Position::new(100, 50),
        };

        assert_eq!(window, window.clip_to(Resolution::new(1920, 1080)).unwrap());
    }

    #[test]
    fn clip_trims_a_window_hanging_off_the_edges() {
        let window = WindowInfo {
            resolution: Resolution::new(800, 600),
            position:   Position::new(-100, 900),
        };

        assert_eq!(
            WindowInfo {
                resolution: Resolution::new(700, 180), position: Position::new(0, 900)
            },
            window.clip_to(Resolution::new(1920, 1080)).unwrap()
        );
    }
}
