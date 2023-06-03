use std::{future::Future, pin::Pin};
use tokio::sync::oneshot;

pub struct AsyncTask<T> {
    fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    receiver: AsyncReceiver<T>,
}

impl<T: Send + Sync + 'static> AsyncTask<T> {
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = fut.await;
            _ = tx.send(result);
        };
        let fut = Box::pin(new_fut);
        let receiver = AsyncReceiver { buffer: rx };
        Self { fut, receiver }
    }

    /// Block awaiting the task result. Can only be used outside of async contexts.
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(self) -> T {
        let (fut, rx) = self.into_parts();
        futures::executor::block_on(fut);
        rx.buffer.blocking_recv().unwrap()
    }

    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        AsyncReceiver<T>,
    ) {
        (self.fut, self.receiver)
    }
}

pub struct AsyncReceiver<T> {
    buffer: oneshot::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Block the current thread waiting for the async result.
    ///
    /// # Panics
    /// Panics if the sender was dropped without sending.
    pub fn try_recv(&mut self) -> Option<T> {
        match self.buffer.try_recv() {
            Ok(t) => Some(t),
            Err(oneshot::error::TryRecvError::Empty) => None,
            _ => panic!("the sender was dropped without sending"),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::AsyncTask;
    use futures::{pin_mut, select, FutureExt};
    use futures_timer::Delay;
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            if tx.send(3).is_err() {
                panic!("the receiver dropped");
            }
        });

        match rx.await {
            Ok(v) => assert_eq!(3, v),
            Err(_) => panic!("the sender dropped"),
        }
    }

    #[test]
    fn test_blocking_recv() {
        let task = AsyncTask::new(async move { 5 });
        assert_eq!(5, task.blocking_recv());
    }

    #[tokio::test]
    async fn test_try_recv() {
        let task = AsyncTask::new(async move { 5 });
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let fetch = Delay::new(Duration::from_millis(1)).fuse();
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);
        select! {
            _ = &mut fetch => {
                if let Some(v) = rx.try_recv() {
                    assert_eq!(5, v);
                } else {
                    // Reset the clock
                    fetch.set(Delay::new(Duration::from_millis(1)).fuse());
                }
            }
            _ = timeout => panic!("timeout")
        };
    }
}
