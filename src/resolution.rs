use std::process::Stdio;

use anyhow::{anyhow, Context};
use execute::{command, Execute};

#[derive(Debug)]
pub struct Resolution {
    pub width:  i32,
    pub height: i32,
}

impl Resolution {
    #[inline]
    pub fn new(width: i32, height: i32) -> Resolution {
        Resolution {
            width,
            height,
        }
    }

    #[inline]
    pub fn get_screen_resolution() -> anyhow::Result<Resolution> {
        let mut command1 = command!("xrandr");
        let mut command2 = command!("head -n 1");
        let mut command3 = command!("cut -d ',' -f 2");
        let mut command4 = command!("cut -d ' ' -f 3-5");
        let mut command5 = command!("tr -d ' '");

        command5.stdout(Stdio::piped());

        let res_output = command1
            .execute_multiple_output(&mut [
                &mut command2,
                &mut command3,
                &mut command4,
                &mut command5,
            ])
            .with_context(|| anyhow!("xrandr"))?;

        let res = unsafe { String::from_utf8_unchecked(res_output.stdout) };

        let res_tokens: Vec<&str> = res.trim_end().split('x').collect();

        let mut res: Vec<i32> = Vec::with_capacity(2);

        for res_token in res_tokens {
            res.push(res_token.trim().parse().unwrap());
        }

        Ok(Resolution::new(res[0], res[1]))
    }

    #[inline]
    pub fn get_normalized_resolution(&self) -> Resolution {
        let mut width = 7680;
        let mut height = 4320;

        for wh in [
            (3840, 2160),
            (2560, 1440),
            (1920, 1080),
            (1280, 720),
            (854, 480),
            (640, 360),
            (426, 240),
        ]
        .iter()
        {
            if self.width <= wh.0 && self.height <= wh.1 {
                width = wh.0;
                height = wh.1;
            }
        }

        Resolution::new(width, height)
    }
}
