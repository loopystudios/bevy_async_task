use crate::AsyncReceiver;
use futures::channel::oneshot;
use std::{future::Future, pin::Pin};

/// A wrapper type around an async future. The future may be executed
/// asynchronously by an [`AsyncTaskRunner`](crate::AsyncTaskRunner) or
/// [`AsyncTaskPool`](crate::AsyncTaskPool), or it may be blocked on the current
/// thread.
pub struct AsyncTask<T> {
    fut: Pin<Box<dyn Future<Output = ()> + 'static>>,
    receiver: AsyncReceiver<T>,
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
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel();

        // Spawn
        wasm_bindgen_futures::spawn_local(async move {
            if tx.send(3).is_err() {
                panic!("the receiver dropped");
            }

            match rx.await {
                Ok(v) => assert_eq!(3, v),
                Err(e) => panic!("the sender dropped ({e})"),
            }
        });
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

        // Spawn
        wasm_bindgen_futures::spawn_local(async move {
            fut.await;
            assert_eq!(Some(5), rx.try_recv());
        });
    }
}
