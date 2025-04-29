/// A timeout occurred on a timed asyncronous task.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[error("the future has timed out")]
pub struct TimeoutError;
