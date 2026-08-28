use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc,
    },
    thread,
    time::Duration,
};

use clap::Parser;
use frameplay_lib::{Frameplay, FrameplayOptions};
use ratatui::crossterm::event;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(about, version)]
struct Cli {
    pub frame_file: PathBuf,
    #[arg(short, long, default_value_t = 10)]
    pub frame_rate: u32,
}

enum Event {
    Frame(String),
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct FrameFileContent {
    pub name: Option<String>,
    pub frames: Vec<String>,
}

fn read_frame_file(file_path: &PathBuf) -> Result<FrameFileContent, io::Error> {
    let mut content = String::new();
    File::open(file_path)?.read_to_string(&mut content)?;

    let content_struct = serde_json::from_str(&content)?;
    Ok(content_struct)
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let frame_file = read_frame_file(&args.frame_file)?;
    let frames = frame_file.frames;

    let frame_rate = args.frame_rate;

    let (tx, rx) = channel();
    let tx2 = tx.clone();

    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop2 = should_stop.clone();

    let frame_thread = thread::spawn(move || {
        let frameplay = Frameplay::new(
            frames,
            FrameplayOptions {
                frame_rate,
                ..Default::default()
            },
        );

        let frame_period = Duration::from_secs(1) / frame_rate;

        loop {
            tx.send(Event::Frame(frameplay.get_frame().to_string()))?;

            if should_stop2.load(Ordering::Relaxed) {
                break Ok::<(), anyhow::Error>(());
            }

            thread::sleep(frame_period);
        }
    });

    let term_event_thread = thread::spawn(move || loop {
        if event::read()?.is_key_press() {
            tx2.send(Event::Quit)?;
            break Ok::<(), anyhow::Error>(());
        }
    });

    ratatui::run(|rat| {
        let mut frame_str = String::new();

        loop {
            rat.draw(|frame| frame.render_widget(frame_str.as_str(), frame.area()))?;

            match rx.recv()? {
                Event::Frame(new_frame_str) => frame_str = new_frame_str,
                Event::Quit => break Ok::<(), anyhow::Error>(()),
            }
        }
    })?;

    should_stop.store(true, Ordering::Relaxed);

    let _ = frame_thread.join().unwrap();
    let _ = term_event_thread.join().unwrap();

    Ok(())
}
