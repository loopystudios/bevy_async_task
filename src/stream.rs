use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use bevy_tasks::ConditionalSend;
use bevy_tasks::futures_lite::Stream;
use bevy_tasks::futures_lite::StreamExt;
use futures::task::AtomicWaker;
use tokio::sync::mpsc;

use crate::receiver::AsyncStreamReceiver;

/// An async stream task that yields multiple items.
pub struct AsyncStream<T> {
    stream: Pin<Box<dyn Stream<Item = T> + Send + 'static>>,
}

impl<T: ConditionalSend + 'static> AsyncStream<T> {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = T> + Send + 'static,
    {
        Self {
            stream: Box::pin(stream),
        }
    }

    pub fn split(
        self,
    ) -> (
        impl std::future::Future<Output = ()> + Send,
        AsyncStreamReceiver<T>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);
        let waker = Arc::new(AtomicWaker::new());
        let waker_clone = Arc::clone(&waker);

        let fut = async move {
            let mut stream = self.stream;
            while let Some(item) = stream.next().await {
                if tx.send(item).is_err() {
                    break;
                }
            }
            finished_clone.store(true, Ordering::Relaxed);
            waker_clone.wake();
        };

        (
            fut,
            AsyncStreamReceiver {
                finished,
                waker,
                receiver: rx,
            },
        )
    }
}

impl<T, S> From<S> for AsyncStream<T>
where
    T: ConditionalSend + 'static,
    S: Stream<Item = T> + Send + 'static,
{
    fn from(stream: S) -> Self {
        Self::new(stream)
    }
}
