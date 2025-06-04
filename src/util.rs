use futures_timer::Delay;
use web_time::Duration;

/// Never resolves to a value or becomes ready.
pub async fn pending<T>() {
    std::future::pending::<T>().await;
}

/// Sleep for a specified duration.
pub async fn sleep(duration: Duration) {
    Delay::new(duration).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::timeout;
#[cfg(target_arch = "wasm32")]
pub use wasm::timeout;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use futures::FutureExt;
    use gloo_timers::future::TimeoutFuture;
    use web_time::Duration;

    use crate::TimeoutError;

    /// Execute a future or error on timeout, whichever comes first.
    ///
    /// # Errors
    /// Will return `Err` if the timeout occurs before the future is ready.
    pub async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
    where
        F: Future<Output = T>,
    {
        futures::select! {
            res = f.fuse() => Ok(res),
            _ = {
                #[expect(clippy::cast_possible_truncation, reason = "Max timeout is u32::MAX")]
                TimeoutFuture::new(dur.as_millis() as u32).fuse()
            } => Err(TimeoutError),

        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use futures::FutureExt;
    use futures_timer::Delay;
    use web_time::Duration;

    use crate::TimeoutError;

    /// Execute a future or error on timeout, whichever comes first.
    ///
    /// # Errors
    /// Will return `Err` if the timeout occurs before the future is ready.
    pub async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
    where
        F: Future<Output = T>,
    {
        futures::select! {
            res = f.fuse() => Ok(res),
            _ = Delay::new(dur).fuse() => Err(TimeoutError),
        }
    }
}
