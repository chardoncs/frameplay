use std::time::Duration;

pub struct FrameplayOptions {
    pub frame_time_reference: FrameTimeReference,
    pub frame_rate: u32,
}

impl Default for FrameplayOptions {
    fn default() -> Self {
        Self {
            frame_time_reference: FrameTimeReference::default(),
            frame_rate: 10,
        }
    }
}

#[derive(Default)]
pub enum FrameTimeReference {
    Absolute,
    StartTime,
    #[default]
    Relative,
    Custom(Duration),
}
