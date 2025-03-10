use futures::task::AtomicWaker;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot::{self};

/// A channel that catches an [`AsyncTask`](crate::AsyncTask) result.
#[derive(Debug)]
pub struct AsyncReceiver<T> {
    pub(crate) received: Arc<AtomicBool>,
    pub(crate) waker: Arc<AtomicWaker>, // Waker to wake the sender
    pub(crate) receiver: oneshot::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Poll the current thread waiting for the async result.
    pub fn try_recv(&mut self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(t) => {
                self.receiver.close();
                self.received.store(true, Ordering::Relaxed);
                self.waker.wake(); // Wake the sender to drop
                Some(t)
            }
            Err(_) => None,
        }
    }
}
