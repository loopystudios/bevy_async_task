use async_std::future::{timeout, TimeoutError};
use std::{future::Future, pin::Pin};
use tokio::sync::oneshot;
use tokio::time::Duration;

pub struct AsyncTimeoutTask<T>(AsyncTask<Result<T, TimeoutError>>);

impl<T: Send + Sync + 'static> AsyncTimeoutTask<T> {
    pub fn new<F>(dur: Duration, fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let new_fut = async move { timeout(dur, fut).await };
        Self(AsyncTask::new(new_fut))
    }

    /// Block awaiting the task result. Can only be used outside of async contexts.
    ///
    /// # Errors
    /// Returns a timeout error if the timeout was reached
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(self) -> Result<T, TimeoutError> {
        let (fut, rx) = self.0.into_parts();
        futures::executor::block_on(fut);
        rx.buffer.blocking_recv().unwrap()
    }

    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        AsyncReceiver<Result<T, TimeoutError>>,
    ) {
        self.0.into_parts()
    }
}

impl<T> AsyncTimeoutTask<T> {
    /// Poll the current thread waiting for the async result.
    ///
    /// # Panics
    /// Panics if the sender was dropped without sending.
    pub fn try_recv(&mut self) -> Option<Result<T, TimeoutError>> {
        self.0.receiver.try_recv()
    }
}

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
    /// Poll the current thread waiting for the async result.
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
    use super::*;
    use async_std::task::sleep;
    use futures::{pin_mut, FutureExt};
    use futures_timer::Delay;
    use tokio::select;

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

    #[tokio::test]
    async fn test_timeout_task() {
        let task = AsyncTimeoutTask::new(Duration::from_millis(1), async move {
            sleep(Duration::from_millis(100)).await;
            5
        });
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let timeout = Delay::new(Duration::from_millis(5)).fuse();
        pin_mut!(timeout);
        select! {
            _ = timeout => {
                _ = rx.try_recv().unwrap().unwrap_err();
            }
        };
    }

    #[tokio::test]
    async fn test_timed_task() {
        let task = AsyncTimeoutTask::new(Duration::from_millis(100), async move {
            sleep(Duration::from_millis(1)).await;
            5
        });
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let timeout = Delay::new(Duration::from_millis(5)).fuse();
        pin_mut!(timeout);
        select! {
            _ = timeout => {
                assert_eq!(5, rx.try_recv().unwrap().unwrap());
            }
        };
    }
}
