use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use options::*;

mod options;
pub mod ticker;

#[derive(Default)]
pub struct Frameplay<T> {
    frames: Vec<T>,
    pivot: u128,

    frame_time_reference: FrameTimeReference,
    frame_period: u32,
    frame_rate: u32,
}

impl<T> Frameplay<T> {
    pub fn new(value: impl IntoIterator<Item = T>, opts: FrameplayOptions) -> Self {
        Self {
            frames: value.into_iter().collect(),
            pivot: Self::get_frame_time(&opts.frame_time_reference),

            frame_time_reference: opts.frame_time_reference,
            frame_period: 1000u32.checked_div(opts.frame_rate).unwrap_or(0),
            frame_rate: opts.frame_rate,
        }
    }

    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    fn get_frame_time(reference: &FrameTimeReference) -> u128 {
        match reference {
            FrameTimeReference::Absolute => 0,
            FrameTimeReference::StartTime => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis(),
            FrameTimeReference::Custom(ts) => *ts,
        }
    }

    pub fn get_frame(&self) -> &T {
        let cur_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis();

        let index = (((cur_time.wrapping_sub(self.pivot)) as usize)
            .checked_div(self.frame_period as usize)
            .unwrap_or(0))
            % self.frames.len();

        &self.frames[index]
    }

    pub fn reset(&mut self) {
        self.pivot = Self::get_frame_time(&self.frame_time_reference);
    }

    pub fn frame_period(&self) -> u32 {
        self.frame_period
    }
}

impl<I, T> From<I> for Frameplay<T>
where
    I: IntoIterator<Item = T>,
{
    fn from(value: I) -> Self {
        Self::new(value, FrameplayOptions::default())
    }
}
