//! Ergonomic abstractions to async programming in Bevy for all platforms.

mod error;
mod receiver;
mod task;
mod task_pool;
mod task_runner;
mod util;

pub use error::TimeoutError;
pub use receiver::AsyncReceiver;
pub use task::AsyncTask;
pub use task::TimedAsyncTask;
pub use task_pool::TaskPool;
pub use task_pool::TimedTaskPool;
pub use task_runner::TaskRunner;
pub use task_runner::TimedTaskRunner;
pub use util::pending;
pub use util::sleep;
pub use util::timeout;

/// A good default timeout. Values over `i32::MAX` will cause panics on web.
pub const DEFAULT_TIMEOUT: core::time::Duration = core::time::Duration::from_millis(60 * 1000);

/// The maximum timeout allowed. Anything above this will cause a panic on web.
pub const MAX_TIMEOUT: core::time::Duration = {
    // See: https://stackoverflow.com/questions/3468607/why-does-settimeout-break-for-large-millisecond-delay-values
    static I32_MAX_U64: u64 = 2_147_483_647;
    core::time::Duration::from_millis(I32_MAX_U64)
};
