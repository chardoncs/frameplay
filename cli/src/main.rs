use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

use clap::Parser;
use frameplay::{ticker::AsyncTicker, Frameplay, FrameplayOptions};
use ratatui::crossterm::event;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Parser, Debug)]
#[command(about, version)]
struct Cli {
    pub frame_file: PathBuf,
    #[arg(short, long, default_value_t = 10)]
    pub frame_rate: u32,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let frame_file = read_frame_file(&args.frame_file)?;
    let frames = frame_file.frames;

    let frame_rate = args.frame_rate;

    let (frame_tx, mut frame_rx) = broadcast::channel::<String>(frame_rate as usize);
    let (quit_tx, mut quit_rx) = broadcast::channel::<()>(1);

    let mut ticker = AsyncTicker::new(Frameplay::new(
        frames,
        FrameplayOptions {
            frame_rate,
            frame_time_reference: frameplay::FrameTimeReference::StartTime,
            ..Default::default()
        },
    ));
    let ticker_task = tokio::spawn(async move {
        ticker.run(frame_tx).await;
    });

    let term_event_task = tokio::spawn(async move {
        loop {
            if event::read()?.is_key_press() {
                let _ = quit_tx.send(());
                break Ok::<(), anyhow::Error>(());
            }
        }
    });

    let mut terminal = ratatui::init();
    let mut frame_str = String::new();

    let result: anyhow::Result<()> = loop {
        terminal.draw(|f| f.render_widget(frame_str.as_str(), f.area()))?;

        tokio::select! {
            _ = quit_rx.recv() => break Ok(()),
            frame = frame_rx.recv() => {
                match frame {
                    Ok(new_frame) => frame_str = new_frame,
                    Err(broadcast::error::RecvError::Closed) => break Ok(()),
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        }
    };

    ratatui::restore();

    ticker_task.abort();
    term_event_task.abort();

    result
}
