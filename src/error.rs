/// Errors that may occur from running asynchronous tasks.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskError {
    /// Timeout occurred.
    #[error(transparent)]
    Timeout(#[from] async_std::future::TimeoutError),
}
