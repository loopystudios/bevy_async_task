use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use futures::task::AtomicWaker;
use tokio::sync::mpsc;
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

/// A channel that receives multiple items from an async stream.
#[derive(Debug)]
pub struct AsyncStreamReceiver<T> {
    pub(crate) finished: Arc<AtomicBool>,
    pub(crate) waker: Arc<AtomicWaker>,
    pub(crate) receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> AsyncStreamReceiver<T> {
    /// Returns whether the stream has finished producing items.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    /// Try to receive the next item from the stream without blocking.
    pub fn try_recv(&mut self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(item) => Some(item),
            Err(_) => None,
        }
    }
}
