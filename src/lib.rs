#![deny(missing_docs)]
//! Ergonomic abstractions to async programming in Bevy for all platforms.

mod receiver;
mod task;
mod task_pool;
mod task_runner;

pub use receiver::AsyncReceiver;
pub use task::{AsyncTask, TimeoutError};
pub use task_pool::AsyncTaskPool;
pub use task_runner::AsyncTaskRunner;

/// A poll status for an [`AsyncTask`].
pub enum AsyncTaskStatus<T> {
    /// No task is currently being polled.
    Idle,
    /// The task is currently working.
    Pending,
    /// The task is finished.
    Finished(T),
}
