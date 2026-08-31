use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use options::*;

mod options;
pub mod ticker;

/// A `Frameplay` is a repeater of a set of frames.
#[derive(Default)]
pub struct Frameplay<T> {
    frames: Vec<T>,
    pivot: Duration,

    frame_time_reference: FrameTimeReference,
    frame_period: Duration,
    frame_rate: u32,
}

impl<T> Frameplay<T> {
    pub fn new(value: impl IntoIterator<Item = T>, opts: FrameplayOptions) -> Self {
        Self {
            frames: value.into_iter().collect(),
            pivot: Self::get_frame_time(&opts.frame_time_reference),

            frame_time_reference: opts.frame_time_reference,
            frame_period: Duration::from_millis(
                1000u32.checked_div(opts.frame_rate.max(1)).unwrap_or(0).into(),
            ),
            frame_rate: opts.frame_rate,
        }
    }

    fn get_frame_time(reference: &FrameTimeReference) -> Duration {
        match reference {
            FrameTimeReference::Absolute => Duration::ZERO,
            FrameTimeReference::StartTime => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO),
            FrameTimeReference::Custom(ts) => ts.clone(),
        }
    }

    pub fn get_frame(&self) -> &T {
        let cur_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);

        let frame_index = ((cur_time - self.pivot)
            .as_millis()
            .checked_div(self.frame_period.as_millis())
            .unwrap_or(0) as usize)
            % self.frames.len();

        &self.frames[frame_index]
    }

    pub fn reset(&mut self) {
        self.pivot = Self::get_frame_time(&self.frame_time_reference);
    }

    pub fn frame_period(&self) -> &Duration {
        &self.frame_period
    }

    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
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
