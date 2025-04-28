use crate::{AsyncReceiver, Duration, TaskError};
#[cfg(not(target_arch = "wasm32"))]
use async_compat::CompatExt;
use async_std::future::timeout;
use bevy::tasks::{ConditionalSend, ConditionalSendFuture};
use futures::task::AtomicWaker;
use std::{
    fmt::Debug,
    future::{Future, pending},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
};
use tokio::sync::oneshot;

/// A wrapper type around an async future with a timeout. The future may be executed
/// asynchronously by an [`TimedTaskRunner`](crate::TimedTaskRunner) or
/// [`TimedTaskPool`](crate::TimedTaskPool) bevy system parameter.
pub struct TimedAsyncTask<T: ConditionalSend> {
    fut: Pin<Box<dyn ConditionalSendFuture<Output = T> + 'static>>,
    timeout: Duration,
}

impl<T> Debug for TimedAsyncTask<T>
where
    T: Debug + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedAsyncTask")
            .field("fut", &"<future>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`TaskRunner`](crate::TaskRunner) or
/// [`TaskPool`](crate::TaskPool) bevy system parameter.
pub struct AsyncTask<T: ConditionalSend> {
    fut: Pin<Box<dyn ConditionalSendFuture<Output = T> + 'static>>,
}

impl<T> Debug for AsyncTask<T>
where
    T: Debug + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncTask")
            .field("fut", &"<future>")
            .finish()
    }
}

impl<T> AsyncTask<T>
where
    T: ConditionalSend + 'static,
{
    /// Create an async task from a future.
    pub fn new<F>(fut: F) -> Self
    where
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        Self { fut: Box::pin(fut) }
    }

    /// Never resolves to a value or becomes ready.
    pub fn pending() -> Self {
        Self::new(pending())
    }

    /// Split the task into a runnable future and receiver.
    /// This is a low-level operation and only useful for specific needs.
    #[must_use]
    pub fn split(self) -> (Pin<Box<impl Future<Output = ()>>>, AsyncReceiver<T>) {
        let (tx, rx) = oneshot::channel();
        let waker = Arc::new(AtomicWaker::new());
        let received = Arc::new(AtomicBool::new(false));
        let fut = {
            let waker = waker.clone();
            let received = received.clone();
            async move {
                #[cfg(target_arch = "wasm32")]
                let result = self.fut.await;
                #[cfg(not(target_arch = "wasm32"))]
                let result = self.fut.compat().await;

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

    /// Add a timeout for this task.
    #[must_use]
    pub fn with_timeout(self, dur: Duration) -> TimedAsyncTask<T> {
        TimedAsyncTask {
            fut: self.fut,
            timeout: dur,
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

impl<T> TimedAsyncTask<T>
where
    T: ConditionalSend + 'static,
{
    /// Create an async task from a future with a timeout.
    pub fn new<F>(dur: Duration, fut: F) -> Self
    where
        F: ConditionalSendFuture<Output = T> + 'static,
        F::Output: ConditionalSend + 'static,
    {
        Self {
            fut: Box::pin(fut),
            timeout: dur,
        }
    }

    /// Never resolves to a value or becomes ready.
    pub fn pending() -> Self {
        Self::new(crate::DEFAULT_TIMEOUT, pending())
    }

    /// Split the task into a runnable future and receiver.
    /// This is a low-level operation and only useful for specific needs.
    #[must_use]
    pub fn split(
        self,
    ) -> (
        Pin<Box<impl Future<Output = ()>>>,
        AsyncReceiver<Result<T, TaskError>>,
    ) {
        let (tx, rx) = oneshot::channel();
        let waker = Arc::new(AtomicWaker::new());
        let received = Arc::new(AtomicBool::new(false));
        let fut = {
            let waker = waker.clone();
            let received = received.clone();
            async move {
                #[cfg(target_arch = "wasm32")]
                let result = timeout(self.timeout, self.fut)
                    .await
                    .map_err(TaskError::Timeout);
                #[cfg(not(target_arch = "wasm32"))]
                let result = timeout(self.timeout, self.fut.compat())
                    .await
                    .map_err(TaskError::Timeout);

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

    /// Replace the timeout for this task.
    #[must_use]
    pub fn with_timeout(mut self, dur: Duration) -> Self {
        self.timeout = dur;
        self
    }

    /// Remove the timeout for this task.
    #[must_use]
    pub fn without_timeout(self) -> AsyncTask<T> {
        AsyncTask { fut: self.fut }
    }
}

impl<T, Fnc> From<Fnc> for TimedAsyncTask<T>
where
    Fnc: ConditionalSendFuture<Output = T> + 'static,
    Fnc::Output: ConditionalSend + 'static,
{
    fn from(value: Fnc) -> Self {
        Self::new(crate::DEFAULT_TIMEOUT, value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use super::*;
    use futures::{FutureExt, pin_mut};
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
        let (fut, mut rx) = task.split();

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
        let task = TimedAsyncTask::new(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.split();

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
        let task = TimedAsyncTask::new(Duration::from_millis(5), pending::<()>());
        let (fut, mut rx) = task.split();

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
        let (fut, mut rx) = task.split();

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
        assert_eq!(Some(5), rx.try_recv());
    }

    #[wasm_bindgen_test]
    async fn test_timeout() {
        let task = TimedAsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
        let (fut, mut rx) = task.split();

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
        let task = TimedAsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
        let (fut, mut rx) = task.split();

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
