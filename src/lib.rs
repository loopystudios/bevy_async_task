//! Ergonomic abstractions to async programming in Bevy for all platforms.

mod error;
mod receiver;
mod task;
mod task_pool;
mod task_runner;

pub use error::TaskError;
pub use receiver::AsyncReceiver;
pub use task::AsyncTask;
pub use task_pool::AsyncTaskPool;
pub use task_runner::AsyncTaskRunner;
