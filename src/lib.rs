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
                1000u32
                    .checked_div(opts.frame_rate.max(1))
                    .unwrap_or(0)
                    .into(),
            ),
            frame_rate: opts.frame_rate,
        }
    }

    fn get_frame_time(reference: &FrameTimeReference) -> Duration {
        match reference {
            FrameTimeReference::Absolute => Duration::ZERO,
            FrameTimeReference::StartTime => Self::frame_clock(),
            FrameTimeReference::Custom(ts) => *ts,
        }
    }

    fn frame_clock() -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
    }

    pub fn get_frame(&self) -> &T {
        let cur_time = Self::frame_clock();

        let frame_index = ((cur_time.saturating_sub(self.pivot))
            .as_millis()
            .checked_div(self.frame_period.as_millis())
            .unwrap_or(0) as usize)
            % self.frames.len();

        &self.frames[frame_index]
    }

    /// Duration until the frame advances to the next one, or `None` when the
    /// frame never changes (fewer than two frames or a zero frame period).
    pub fn time_to_next_frame(&self) -> Option<Duration> {
        if self.frames.len() < 2 || self.frame_period.is_zero() {
            return None;
        }

        let cur_time = Self::frame_clock();
        let elapsed = cur_time.saturating_sub(self.pivot).as_millis();
        let period = self.frame_period.as_millis();

        Some(Duration::from_millis((period - elapsed % period) as u64))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn options(frame_rate: u32, pivot: FrameTimeReference) -> FrameplayOptions {
        FrameplayOptions {
            frame_time_reference: pivot,
            frame_rate,
        }
    }

    #[test]
    fn wakes_when_the_frame_advances() {
        let fp = Frameplay::new(
            ["a", "b"],
            options(1, FrameTimeReference::Custom(Duration::ZERO)),
        );

        let wait = fp.time_to_next_frame().unwrap();
        assert!(Duration::ZERO < wait && wait <= Duration::from_secs(1));

        let before = *fp.get_frame();
        std::thread::sleep(wait + Duration::from_millis(2));
        assert_ne!(*fp.get_frame(), before);
    }

    #[test]
    fn future_pivot_waits_a_full_period() {
        let pivot = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            + Duration::from_secs(60);
        let fp = Frameplay::new(["a", "b"], options(2, FrameTimeReference::Custom(pivot)));

        assert_eq!(fp.time_to_next_frame(), Some(Duration::from_millis(500)));
    }

    #[test]
    fn single_frame_never_advances() {
        let fp = Frameplay::new(["a"], options(4, FrameTimeReference::Absolute));

        assert_eq!(fp.time_to_next_frame(), None);
    }

    #[test]
    fn zero_frame_period_never_advances() {
        let fp = Frameplay::new(["a", "b"], options(2000, FrameTimeReference::Absolute));

        assert_eq!(fp.time_to_next_frame(), None);
    }
}
