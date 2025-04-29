//! Ergonomic abstractions to async programming in Bevy for all platforms.

mod error;
mod receiver;
mod task;
mod task_pool;
mod task_runner;
mod util;

pub use error::TimeoutError;
pub use receiver::AsyncReceiver;
pub use task::{AsyncTask, TimedAsyncTask};
pub use task_pool::{TaskPool, TimedTaskPool};
pub use task_runner::{TaskRunner, TimedTaskRunner};
pub use util::{pending, sleep, timeout};

/// A good default timeout. Values near `u32::MAX` will overflow.
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_millis(u16::MAX as u64);

// Vendor re-exports
pub use web_time::Duration;
