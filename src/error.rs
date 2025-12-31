/// A timeout occurred on a timed asyncronous task.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[error("the future has timed out")]
pub struct TimeoutError;
