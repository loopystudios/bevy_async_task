mod task_pool;
mod task_runner;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::AsyncTask;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::AsyncTask;

pub use task_pool::AsyncTaskPool;
pub use task_runner::AsyncTaskRunner;

/// The status of an [`AsyncTask`].
pub enum AsyncTaskStatus<T> {
    Idle,
    Pending,
    Finished(T),
}

pub struct AsyncReceiver<T> {
    received: bool,
    buffer: futures::channel::oneshot::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Poll the current thread waiting for the async result.
    ///
    /// # Panics
    /// Panics if the sender was dropped without sending
    pub fn try_recv(&mut self) -> Option<T> {
        match self.buffer.try_recv() {
            Ok(Some(t)) => {
                self.received = true;
                self.buffer.close();
                Some(t)
            }
            Ok(None) => None,
            Err(_) => panic!("the sender was dropped without sending"),
        }
    }
}
