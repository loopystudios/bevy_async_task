use crate::AsyncReceiver;
use async_std::future::{timeout, TimeoutError};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::sync::oneshot;

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`AsyncTaskRunner`](crate::AsyncTaskRunner) or
/// [`AsyncTaskPool`](crate::AsyncTaskPool), or it may be blocked on the current
/// thread.
pub struct AsyncTask<T> {
    fut: Pin<Box<dyn Future<Output = ()> + 'static>>,
    receiver: AsyncReceiver<T>,
}

impl<T> AsyncTask<T>
where
    T: 'static,
{
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

impl<T> AsyncTask<T>
where
    T: Send + 'static,
{
    /// Never resolves to a value or finishes.
    pub fn pending() -> AsyncTask<T> {
        AsyncTask::new(async_std::future::pending::<T>())
    }
}

impl<T> AsyncTask<T> {
    /// Create an async task from a future.
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = T> + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = fut.await;
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
        F: Future<Output = T> + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = timeout(dur, fut).await;
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
        Pin<Box<dyn Future<Output = ()> + 'static>>,
        AsyncReceiver<T>,
    ) {
        (self.fut, self.receiver)
    }
}

impl<T, Fnc> From<Fnc> for AsyncTask<T>
where
    Fnc: Future<Output = T> + 'static,
    Fnc::Output: Send + 'static,
{
    fn from(value: Fnc) -> Self {
        AsyncTask::new(value)
    }
}

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
        let task = AsyncTask::new_with_timeout(
            Duration::from_millis(5),
            async_std::future::pending::<()>(),
        );
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
        let task =
            AsyncTask::<()>::pending().with_timeout(Duration::from_millis(5));
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
