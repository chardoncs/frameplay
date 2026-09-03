# Frameplay

A simple frame repeater

> [!NOTE]
>
> Under development

## Usage

```rust
use std::time::Duration;

use frameplay::{FrameTimeReference, Frameplay, FrameplayOptions};

let fp = Frameplay::new(
    ["-", "\\", "|", "/"],
    FrameplayOptions {
        frame_time_reference: FrameTimeReference::StartTime,
        frame_rate: 10,
    },
);

// The frame is a pure function of wall-clock time.
let frame = *fp.get_frame();

// How long until the frame advances — useful to schedule redraws.
let until_next = fp.time_to_next_frame();
```