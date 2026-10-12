use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::{FrameIndexer, Frameplay};

/// A ticker that periodically emits the current frame index.
pub struct IndexerTicker {
    indexer: FrameIndexer,
    frame_period: Duration,
    tx: Option<Sender<usize>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl IndexerTicker {
    pub fn new(indexer: FrameIndexer) -> Self {
        let frame_period = *indexer.frame_period();

        Self {
            indexer,
            frame_period,
            tx: None,
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn start(&mut self) -> Receiver<usize> {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let frame_period = self.frame_period;
        let stop = self.stop.clone();
        let indexer = self.indexer.clone();

        self.handle = Some(thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if thread_tx.send(indexer.get_cur_index()).is_err() {
                    break;
                }
                thread::sleep(frame_period);
            }
        }));

        self.tx = Some(tx);
        rx
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.tx.take();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for IndexerTicker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A `Ticker` repeatedy emits the current frame. It drives an `IndexerTicker`
/// and maps the emitted indices back onto the frames.
pub struct Ticker<T>
where
    T: Clone + Send + 'static,
{
    frames: Vec<T>,
    ticker: IndexerTicker,
    handle: Option<JoinHandle<()>>,
}

impl<T> Ticker<T>
where
    T: Clone + Send + 'static,
{
    pub fn new(fp: Frameplay<T>) -> Self {
        let (indexer, frames) = fp.into_parts();

        Self {
            frames,
            ticker: IndexerTicker::new(indexer),
            handle: None,
        }
    }

    pub fn start(&mut self) -> Receiver<T> {
        let index_rx = self.ticker.start();
        let (tx, rx) = mpsc::channel();
        let frames = std::mem::take(&mut self.frames);

        self.handle = Some(thread::spawn(move || {
            for index in index_rx {
                if tx.send(frames[index].clone()).is_err() {
                    break;
                }
            }
        }));

        rx
    }

    pub fn stop(&mut self) {
        self.ticker.stop();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl<T> Drop for Ticker<T>
where
    T: Clone + Send + 'static,
{
    fn drop(&mut self) {
        self.stop();
    }
}

/// A `Ticker` that periodically emits the current frame index.
#[cfg(feature = "async-tokio")]
pub struct AsyncIndexerTicker {
    indexer: FrameIndexer,
    interval: tokio::time::Interval,
}

#[cfg(feature = "async-tokio")]
impl AsyncIndexerTicker {
    pub fn new(indexer: FrameIndexer) -> Self {
        let interval = tokio::time::interval(*indexer.frame_period());

        Self { indexer, interval }
    }

    /// Wait for the next tick and return the current frame index.
    pub async fn next_index(&mut self) -> usize {
        self.interval.tick().await;
        self.indexer.get_cur_index()
    }

    pub async fn run(&mut self, tx: tokio::sync::broadcast::Sender<usize>) {
        loop {
            let index = self.next_index().await;

            if tx.send(index).is_err() {
                break;
            }
        }
    }
}

/// A `Ticker` that repeatedy emits the current frame. It drives an
/// `AsyncIndexerTicker` and maps the emitted indices back onto the frames.
#[cfg(feature = "async-tokio")]
pub struct AsyncTicker<T>
where
    T: Clone,
{
    frames: Vec<T>,
    ticker: AsyncIndexerTicker,
}

#[cfg(feature = "async-tokio")]
impl<T> AsyncTicker<T>
where
    T: Clone,
{
    pub fn new(fp: Frameplay<T>) -> Self {
        let (indexer, frames) = fp.into_parts();

        Self {
            frames,
            ticker: AsyncIndexerTicker::new(indexer),
        }
    }

    pub async fn run(&mut self, tx: tokio::sync::broadcast::Sender<T>) {
        loop {
            let index = self.ticker.next_index().await;

            if tx.send(self.frames[index].clone()).is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{FrameTimeReference, FrameplayOptions};

    fn options(frame_rate: u32) -> FrameplayOptions {
        FrameplayOptions {
            frame_time_reference: FrameTimeReference::Custom(Duration::ZERO),
            frame_rate,
        }
    }

    #[test]
    fn indexer_ticker_emits_indices_in_range() {
        let mut ticker = IndexerTicker::new(FrameIndexer::new(3, options(100)));

        let rx = ticker.start();
        assert!(rx.recv().unwrap() < 3);

        ticker.stop();
    }

    #[test]
    fn ticker_emits_frames() {
        let mut ticker = Ticker::new(Frameplay::new(["a", "b"], options(100)));

        let rx = ticker.start();
        assert!(matches!(rx.recv().unwrap(), "a" | "b"));

        ticker.stop();
    }
}
