/// The 16:9 sizes that `--normalize` pads up to, ordered from the largest to the smallest.
const NORMALIZED_RESOLUTIONS: [(i32, i32); 8] = [
    (7680, 4320),
    (3840, 2160),
    (2560, 1440),
    (1920, 1080),
    (1280, 720),
    (854, 480),
    (640, 360),
    (426, 240),
];

/// A width and a height, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width:  i32,
    pub height: i32,
}

impl Resolution {
    #[inline]
    pub const fn new(width: i32, height: i32) -> Resolution {
        Resolution {
            width,
            height,
        }
    }

    /// Round the size down to even numbers, which the `yuv420p` pixel format requires.
    #[inline]
    pub const fn to_even(self) -> Resolution {
        Resolution::new(self.width & !1, self.height & !1)
    }

    /// The smallest 16:9 size this resolution fits into, or `None` if it is larger than 8K.
    #[inline]
    pub fn get_normalized_resolution(&self) -> Option<Resolution> {
        NORMALIZED_RESOLUTIONS
            .iter()
            .rev()
            .find(|(width, height)| self.width <= *width && self.height <= *height)
            .map(|&(width, height)| Resolution::new(width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_picks_the_smallest_fitting_size() {
        assert_eq!(
            Some(Resolution::new(1920, 1080)),
            Resolution::new(1920, 1080).get_normalized_resolution()
        );
        assert_eq!(
            Some(Resolution::new(1280, 720)),
            Resolution::new(1000, 700).get_normalized_resolution()
        );
        assert_eq!(
            Some(Resolution::new(854, 480)),
            Resolution::new(500, 400).get_normalized_resolution()
        );
        assert_eq!(
            Some(Resolution::new(3840, 2160)),
            Resolution::new(3440, 1440).get_normalized_resolution()
        );
    }

    #[test]
    fn normalize_rejects_sizes_beyond_8k() {
        assert_eq!(None, Resolution::new(8000, 5000).get_normalized_resolution());
    }

    #[test]
    fn to_even_rounds_down() {
        assert_eq!(Resolution::new(1918, 1080), Resolution::new(1919, 1081).to_even());
        assert_eq!(Resolution::new(1920, 1080), Resolution::new(1920, 1080).to_even());
    }
}
