#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::timeout;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::timeout;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::TimeoutError;
    use futures::FutureExt;
    use gloo_timers::future::TimeoutFuture;
    use web_time::Duration;

    pub(crate) async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
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
    use crate::TimeoutError;
    use futures::FutureExt;
    use futures_timer::Delay;
    use web_time::Duration;

    pub(crate) async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
    where
        F: Future<Output = T>,
    {
        futures::select! {
            res = f.fuse() => Ok(res),
            _ = Delay::new(dur).fuse() => Err(TimeoutError),
        }
    }
}
