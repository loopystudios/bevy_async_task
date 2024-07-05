use crate::AsyncReceiver;
#[cfg(not(target_arch = "wasm32"))]
use async_compat::CompatExt;
use async_std::future::timeout;
use bevy::utils::{ConditionalSend, ConditionalSendFuture};
use std::{future::pending, pin::Pin, time::Duration};
use tokio::sync::oneshot;

// Re-export timeout error
pub use async_std::future::TimeoutError;

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`AsyncTaskRunner`](crate::AsyncTaskRunner) or
/// [`AsyncTaskPool`](crate::AsyncTaskPool), or it may be blocked on the current
/// thread.
pub struct AsyncTask<T> {
    fut: Pin<Box<dyn ConditionalSendFuture<Output = ()> + 'static>>,
    receiver: AsyncReceiver<T>,
}

impl<T> AsyncTask<T>
where
    T: ConditionalSend + 'static,
{
    /// Never resolves to a value or finishes.
    pub fn pending() -> AsyncTask<T> {
        AsyncTask::new(pending())
    }

    /// Add a timeout to the task.
    pub fn with_timeout(mut self, dur: Duration) -> AsyncTask<Result<T, TimeoutError>> {
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
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            #[cfg(target_arch = "wasm32")]
            let result = fut.await;
            #[cfg(not(target_arch = "wasm32"))]
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
    pub fn new_with_timeout<F>(dur: Duration, fut: F) -> AsyncTask<Result<T, TimeoutError>>
    where
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            #[cfg(target_arch = "wasm32")]
            let result = timeout(dur, fut).await;
            #[cfg(not(target_arch = "wasm32"))]
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
    pub fn blocking_recv(self) -> T {
        let (fut, mut rx) = self.into_parts();
        bevy::tasks::block_on(fut);
        rx.buffer.try_recv().unwrap()
    }

    /// Break apart the task into a runnable future and the receiver. The
    /// receiver is used to catch the output when the runnable is polled.
    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Pin<Box<dyn ConditionalSendFuture<Output = ()> + 'static>>,
        AsyncReceiver<T>,
    ) {
        (self.fut, self.receiver)
    }
}

impl<T, Fnc> From<Fnc> for AsyncTask<T>
where
    Fnc: ConditionalSendFuture<Output = T> + 'static,
    Fnc::Output: ConditionalSend + 'static,
{
    fn from(value: Fnc) -> Self {
        AsyncTask::new(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
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
        let task = AsyncTask::new_with_timeout(Duration::from_millis(5), pending::<()>());
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
        let task = AsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
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

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod test {
    use super::*;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel();

        // Async test
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            if tx.send(3).is_err() {
                panic!("the receiver dropped");
            }

            match rx.await {
                Ok(v) => assert_eq!(3, v),
                Err(e) => panic!("the sender dropped ({e})"),
            }

            Ok(JsValue::NULL)
        }))
        .await
        .unwrap();
    }

    #[wasm_bindgen_test]
    fn test_blocking_recv() {
        let task = AsyncTask::new(async move { 5 });
        assert_eq!(5, task.blocking_recv());
    }

    #[wasm_bindgen_test]
    async fn test_try_recv() {
        let task = AsyncTask::new(async move { 5 });
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap();

        // Spawn
        assert_eq!(Some(5), rx.try_recv());
    }

    #[wasm_bindgen_test]
    async fn test_timeout() {
        let task = AsyncTask::new_with_timeout(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap();

        // Spawn
        let v = rx.try_recv().expect("future loaded no value");
        assert!(v.is_err(), "timeout should have triggered!");
    }

    #[wasm_bindgen_test]
    async fn test_with_timeout() {
        let task = AsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap();

        // Spawn
        let v = rx.try_recv().expect("future loaded no value");
        assert!(v.is_err(), "timeout should have triggered!");
    }
}
