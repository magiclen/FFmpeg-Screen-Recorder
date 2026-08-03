/// A point on the X11 root window, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Position {
        Position {
            x,
            y,
        }
    }
}
