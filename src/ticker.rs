use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::Frameplay;

pub struct Ticker<T>
where
    T: Clone + Send + 'static,
{
    fp: Frameplay<T>,
    frame_period: Duration,
    tx: Option<Sender<T>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl<T> Ticker<T>
where
    T: Clone + Send + 'static,
{
    pub fn new(fp: Frameplay<T>) -> Self {
        let frame_rate = fp.frame_rate();

        Self {
            fp,
            frame_period: Duration::from_secs_f64(1.0 / frame_rate.max(1) as f64),
            tx: None,
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn start(&mut self) -> Receiver<T> {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let frame_period = self.frame_period;
        let stop = self.stop.clone();
        let fp = std::mem::replace(&mut self.fp, Frameplay::new(Vec::new(), crate::FrameplayOptions::default()));

        self.handle = Some(thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let frame = fp.get_frame().clone();
                if thread_tx.send(frame).is_err() {
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

impl<T> Drop for Ticker<T>
where
    T: Clone + Send + 'static,
{
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(feature = "async-tokio")]
pub struct AsyncTicker<T>
where
    T: Clone,
{
    fp: Frameplay<T>,
    frame_period: Duration,
}

#[cfg(feature = "async-tokio")]
impl<T> AsyncTicker<T>
where
    T: Clone,
{
    pub fn new(fp: Frameplay<T>) -> Self {
        let frame_rate = fp.frame_rate();

        Self {
            fp,
            frame_period: Duration::from_secs_f64(1.0 / frame_rate.max(1) as f64),
        }
    }

    pub async fn run(&mut self, tx: tokio::sync::broadcast::Sender<T>) {
        let mut interval = tokio::time::interval(self.frame_period);

        loop {
            interval.tick().await;

            if tx.send(self.fp.get_frame().clone()).is_err() {
                break;
            }
        }
    }
}
