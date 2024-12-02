use crate::{AsyncReceiver, Duration, TaskError};
#[cfg(not(target_arch = "wasm32"))]
use async_compat::CompatExt;
use async_std::future::timeout;
use bevy::utils::{ConditionalSend, ConditionalSendFuture};
use futures::task::AtomicWaker;
use std::{
    fmt::Debug,
    future::{pending, Future},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};
use tokio::sync::oneshot;

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`AsyncTaskRunner`](crate::AsyncTaskRunner) or
/// [`AsyncTaskPool`](crate::AsyncTaskPool) bevy system parameter.
pub struct AsyncTask<T: ConditionalSend> {
    fut: Pin<Box<dyn ConditionalSendFuture<Output = T> + 'static>>,
    timeout: Option<Duration>,
}

impl<T> Debug for AsyncTask<T>
where
    T: Debug + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncTask")
            .field("fut", &"<future>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl<T> AsyncTask<T>
where
    T: ConditionalSend + 'static,
{
    /// Never resolves to a value or finishes.
    pub fn pending() -> Self {
        Self::new(pending())
    }
}

impl<T> AsyncTask<T>
where
    T: ConditionalSend,
{
    /// Build the task into a runnable future and receiver.
    /// This is a low-level operation and only useful for specific needs.
    #[must_use]
    pub fn build(self) -> (Pin<Box<impl Future<Output = ()>>>, AsyncReceiver<T>) {
        let (tx, rx) = oneshot::channel();
        let waker = Arc::new(AtomicWaker::new());
        let received = Arc::new(AtomicBool::new(false));
        let fut = {
            let waker = waker.clone();
            let received = received.clone();
            async move {
                #[cfg(target_arch = "wasm32")]
                let result = if let Some(dur) = self.timeout {
                    timeout(dur, self.fut).await.map_err(TaskError::Timeout)
                } else {
                    Ok(self.fut.await)
                };
                #[cfg(not(target_arch = "wasm32"))]
                let result = if let Some(dur) = self.timeout {
                    timeout(dur, self.fut.compat())
                        .await
                        .map_err(TaskError::Timeout)
                } else {
                    Ok(self.fut.compat().await)
                };

                if let Ok(()) = tx.send(result) {
                    // Wait for the receiver to get the result before dropping.
                    futures::future::poll_fn(|cx| {
                        waker.register(cx.waker());
                        if received.load(Ordering::Relaxed) {
                            Poll::Ready(())
                        } else {
                            Poll::Pending::<()>
                        }
                    })
                    .await;
                }
            }
        };
        let fut = Box::pin(fut);
        let receiver = AsyncReceiver {
            received,
            waker,
            receiver: rx,
        };
        (fut, receiver)
    }

    /// Create an async task from a future.
    pub fn new<F>(fut: F) -> Self
    where
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        Self {
            fut: Box::pin(fut),
            timeout: None,
        }
    }

    /// Create an async task from a future with a timeout.
    pub fn new_with_timeout<F>(dur: Duration, fut: F) -> Self
    where
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        Self {
            fut: Box::pin(fut),
            timeout: Some(dur),
        }
    }

    /// Replace the timeout for this task.
    #[must_use]
    pub fn with_timeout(self, dur: Duration) -> Self {
        Self {
            timeout: Some(dur),
            ..self
        }
    }
}

impl<T, Fnc> From<Fnc> for AsyncTask<T>
where
    Fnc: ConditionalSendFuture<Output = T> + 'static,
    Fnc::Output: ConditionalSend + 'static,
{
    fn from(value: Fnc) -> Self {
        Self::new(value)
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

    #[tokio::test]
    async fn test_try_recv() {
        let task = AsyncTask::new(async move { 5 });
        let (fut, mut rx) = task.build();

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
                        assert_eq!(5, v.unwrap());
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
        let (fut, mut rx) = task.build();

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
                        if matches!(v, Err(TaskError::Timeout(_))) {
                            // Good ending
                            break 'result;
                        } else {
                            panic!("timeout should have triggered!");
                        }
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
        let task = AsyncTask::new_with_timeout(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.build();

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
                        if matches!(v, Err(TaskError::Timeout(_))) {
                            // Good ending
                            break 'result;
                        } else {
                            panic!("timeout should have triggered!");
                        }
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
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });
    }

    #[wasm_bindgen_test]
    async fn test_try_recv() {
        let task = AsyncTask::new(async move { 5 });
        let (fut, mut rx) = task.build();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Spawn
        assert_eq!(Some(Ok(5)), rx.try_recv());
    }

    #[wasm_bindgen_test]
    async fn test_timeout() {
        let task = AsyncTask::new_with_timeout(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.build();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Spawn
        let v = rx.try_recv().unwrap_or_else(|| {
            panic!("expected result after await");
        });
        assert!(v.is_err(), "timeout should have triggered!");
    }

    #[wasm_bindgen_test]
    async fn test_with_timeout() {
        let task = AsyncTask::new_with_timeout(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.build();

        assert_eq!(None, rx.try_recv());

        // Convert to Promise and -await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Spawn
        let v = rx.try_recv().unwrap_or_else(|| {
            panic!("expected result after await");
        });
        assert!(matches!(v, Err(TaskError::Timeout(_))), "");
        assert!(v.is_err(), "timeout should have triggered!");
    }
}
