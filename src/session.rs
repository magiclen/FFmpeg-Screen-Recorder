use std::env;

use anyhow::anyhow;

/// The X11 display to capture, taken from `--display` or from the `DISPLAY` environment variable.
pub fn resolve_display(requested: Option<&str>) -> String {
    match requested {
        Some(display) => display.to_string(),
        None => env::var("DISPLAY").unwrap_or_else(|_| String::from(":0")),
    }
}

/// Refuse to record a Wayland session, where x11grab only ever captures an empty XWayland screen.
pub fn check_x11_session(display_was_requested: bool) -> anyhow::Result<()> {
    // An explicit --display means the user knows which X server to record, so their word is taken for it.
    if display_was_requested || !is_wayland() {
        return Ok(());
    }

    Err(anyhow!(
        "this looks like a Wayland session, where x11grab can only capture an empty XWayland \
         screen.\nThis program records X11 only. Log in to an X11/Xorg session, or pass --display \
         if you really do have an X server to record."
    ))
}

fn is_wayland() -> bool {
    env::var("XDG_SESSION_TYPE").is_ok_and(|session| session.eq_ignore_ascii_case("wayland"))
        || env::var("WAYLAND_DISPLAY").is_ok_and(|display| !display.is_empty())
}
