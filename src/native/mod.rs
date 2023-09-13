use crate::AsyncReceiver;
use async_compat::CompatExt;
use async_std::future::{timeout, TimeoutError};
use futures::channel::oneshot;
use std::{future::Future, pin::Pin, time::Duration};

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`AsyncTaskRunner`](crate::AsyncTaskRunner) or
/// [`AsyncTaskPool`](crate::AsyncTaskPool), or it may be blocked on the current
/// thread.
pub struct AsyncTask<T> {
    fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    receiver: AsyncReceiver<T>,
}

impl<T> AsyncTask<T>
where
    T: Send + 'static,
{
    /// Never resolves to a value or finishes.
    pub fn pending() -> AsyncTask<T> {
        AsyncTask::new(async_std::future::pending::<T>())
    }

    /// Add a timeout to the task.
    pub fn with_timeout(
        mut self,
        dur: Duration,
    ) -> AsyncTask<Result<T, TimeoutError>> {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = timeout(dur, self.fut)
                .await
                .map(|_| self.receiver.try_recv().unwrap());
            _ = tx.send(result);
        };
        let fut = Box::pin(new_fut);
        let receiver = AsyncReceiver {
            received: false,
            buffer: rx,
        };
        AsyncTask::<Result<T, TimeoutError>> { fut, receiver }
    }
}

impl<T> AsyncTask<T> {
    /// Create an async task from a future.
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = fut.compat().await;
            _ = tx.send(result);
        };
        let fut = Box::pin(new_fut);
        let receiver = AsyncReceiver {
            received: false,
            buffer: rx,
        };
        Self { fut, receiver }
    }

    /// Create an async task from a future with a timeout.
    pub fn new_with_timeout<F>(
        dur: Duration,
        fut: F,
    ) -> AsyncTask<Result<T, TimeoutError>>
    where
        F: Future<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = timeout(dur, fut.compat()).await;
            _ = tx.send(result);
        };
        let fut = Box::pin(new_fut);
        let receiver = AsyncReceiver {
            received: false,
            buffer: rx,
        };
        AsyncTask::<Result<T, TimeoutError>> { fut, receiver }
    }

    /// Block awaiting the task result. Can only be used outside of async
    /// contexts.
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(self) -> T {
        let (fut, mut rx) = self.into_parts();
        futures::executor::block_on(fut);
        rx.buffer.try_recv().unwrap().unwrap()
    }

    /// Break apart the task into a runnable future and the receiver. The
    /// receiver is used to catch the output when the runnable is polled.
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

impl<T, Fnc> From<Fnc> for AsyncTask<T>
where
    Fnc: Future<Output = T> + Send + 'static,
    Fnc::Output: Send + 'static,
{
    fn from(value: Fnc) -> Self {
        AsyncTask::new(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{pin_mut, FutureExt};
    use futures_timer::Delay;
    use std::time::Duration;
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
            Err(e) => panic!("the sender dropped ({e})"),
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
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);
        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {

                    assert_eq!(5, v);
                        break 'result;
                    } else {
                        // Reset the clock
                        fetch.reset(Duration::from_millis(1));
                    }
                }
                _ = &mut timeout => panic!("timeout")
            };
        }
    }

    #[tokio::test]
    async fn test_timeout() {
        let task = AsyncTask::new_with_timeout(
            Duration::from_millis(5),
            async_std::future::pending::<()>(),
        );
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);
        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        assert!(v.is_err(), "timeout should have triggered!");
                        break 'result;
                    } else {
                        // Reset the clock
                        fetch.reset(Duration::from_millis(1));
                    }
                }
                _ = &mut timeout => panic!("timeout")
            };
        }
    }

    #[tokio::test]
    async fn test_with_timeout() {
        let task =
            AsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);
        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        assert!(v.is_err(), "timeout should have triggered!");
                        break 'result;
                    } else {
                        // Reset the clock
                        fetch.reset(Duration::from_millis(1));
                    }
                }
                _ = &mut timeout => panic!("timeout")
            };
        }
    }
}
