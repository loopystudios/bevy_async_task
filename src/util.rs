/// Never resolves to a value or becomes ready.
pub async fn pending<T>() {
    std::future::pending::<T>().await;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use futures::FutureExt;
    use gloo_timers::future::TimeoutFuture;
    use web_time::Duration;

    use crate::TimeoutError;

    /// Sleep for a specified duration. This is non-blocking.
    ///
    /// # Panics
    /// This function will panic if the specified Duration is greater than `i32::MAX` in millis.
    pub async fn sleep(dur: Duration) {
        let millis = i32::try_from(dur.as_millis()).unwrap_or_else(|_e| {
            panic!("failed to cast the duration into a i32 with Duration::as_millis.")
        });
        TimeoutFuture::new(millis as u32).await;
    }

    /// Execute a future or error on timeout, whichever comes first.
    ///
    /// # Errors
    /// Will return `Err` if the timeout occurs before the future is ready.
    ///
    /// # Panics
    /// This function will panic if the specified Duration is greater than `i32::MAX` in millis.
    pub async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
    where
        F: Future<Output = T>,
    {
        futures::select! {
            res = f.fuse() => Ok(res),
            _ = {
                let millis = i32::try_from(dur.as_millis()).unwrap_or_else(|_e| {
                    panic!("failed to cast the duration into a i32 with Duration::as_millis.")
                });
                TimeoutFuture::new(millis as u32).fuse()
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

    /// Sleep for a specified duration. This is non-blocking.
    pub async fn sleep(duration: Duration) {
        Delay::new(duration).await;
    }

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
