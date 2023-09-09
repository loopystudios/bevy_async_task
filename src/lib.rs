use cfg_if::cfg_if;

mod receiver;
mod task_pool;
mod task_runner;

pub use receiver::AsyncReceiver;
pub use task_pool::AsyncTaskPool;
pub use task_runner::AsyncTaskRunner;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        mod wasm;
        pub use wasm::AsyncTask;
    } else {
        mod native;
        pub use native::AsyncTask;
    }
}

/// The status of an [`AsyncTask`].
pub enum AsyncTaskStatus<T> {
    /// No task is currently being polled.
    Idle,
    /// The task is currently working.
    Pending,
    /// The task is finished.
    Finished(T),
}
