use bevy::ecs::component::Tick;
use bevy::ecs::system::{ReadOnlySystemParam, SystemMeta, SystemParam};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::utils::synccell::SyncCell;
use std::{future::Future, pin::Pin};
use tokio::sync::oneshot;

/// A task runner which executes [`AsyncTask`]s in the background.
pub struct AsyncTaskRunner<'s, T>(pub(crate) &'s mut Option<AsyncReceiver<T>>);

impl<'s, T> AsyncTaskRunner<'s, T> {
    /// Returns whether the task runner is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    /// Returns whether the task runner is pending (running, but not finished).
    pub fn is_pending(&self) -> bool {
        if let Some(ref rx) = self.0 {
            !rx.received
        } else {
            false
        }
    }

    /// Returns whether the task runner is finished.
    pub fn is_finished(&self) -> bool {
        if let Some(ref rx) = self.0 {
            rx.received
        } else {
            false
        }
    }

    /// Block awaiting the task result. Can only be used outside of async contexts.
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(&mut self, task: impl Into<AsyncTask<T>>) -> T {
        let task = task.into();
        task.blocking_recv()
    }

    /// Run an async task in the background.
    pub fn begin(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.into_parts();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Poll the task runner for the current task status. If no task has begun, this will return `Idle`.
    /// Possible returns are `Idle`, `Pending`, or `Finished(T)`.
    #[must_use]
    pub fn poll(&mut self) -> AsyncTaskStatus<T> {
        match self.0.as_mut() {
            Some(rx) => match rx.try_recv() {
                Some(v) => {
                    self.0.take();
                    AsyncTaskStatus::Finished(v)
                }
                None => AsyncTaskStatus::Pending,
            },
            None => AsyncTaskStatus::Idle,
        }
    }
}

// SAFETY: Only accesses internal state locally, similar to bevy's `Local`
unsafe impl<'s, T: Send + 'static> ReadOnlySystemParam for AsyncTaskRunner<'s, T> {}

// SAFETY: Only accesses internal state locally, similar to bevy's `Local`
unsafe impl<'a, T: Send + 'static> SystemParam for AsyncTaskRunner<'a, T> {
    type State = SyncCell<Option<AsyncReceiver<T>>>;
    type Item<'w, 's> = AsyncTaskRunner<'s, T>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(None)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        AsyncTaskRunner(state.get())
    }
}

/// The status of an [`AsyncTask`].
pub enum AsyncTaskStatus<T> {
    Idle,
    Pending,
    Finished(T),
}

/// A task than may be ran by an [`AsyncTaskRunner`], or broken into parts.
pub struct AsyncTask<T> {
    fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    receiver: AsyncReceiver<T>,
}

impl<T> AsyncTask<T> {
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let new_fut = async move {
            let result = fut.await;
            _ = tx.send(result);
        };
        let fut = Box::pin(new_fut);
        let receiver = AsyncReceiver {
            received: false,
            buffer: rx,
        };
        Self { fut, receiver }
    }

    /// Block awaiting the task result. Can only be used outside of async contexts.
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(self) -> T {
        let (fut, rx) = self.into_parts();
        futures::executor::block_on(fut);
        rx.buffer.blocking_recv().unwrap()
    }

    /// Break apart the task into a runnable future and the receiver. The receiver is used to catch the output when the runnable is polled.
    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        AsyncReceiver<T>,
    ) {
        (self.fut, self.receiver)
    }
}

impl<T, Fnc> From<Fnc> for AsyncTask<T>
where
    Fnc: Future<Output = T> + Send + 'static,
    Fnc::Output: Send + 'static,
{
    fn from(value: Fnc) -> Self {
        AsyncTask::new(value)
    }
}

pub struct AsyncReceiver<T> {
    received: bool,
    buffer: oneshot::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Poll the current thread waiting for the async result.
    ///
    /// # Panics
    /// Panics if the sender was dropped without sending
    pub fn try_recv(&mut self) -> Option<T> {
        match self.buffer.try_recv() {
            Ok(t) => {
                self.received = true;
                self.buffer.close();
                Some(t)
            }
            Err(oneshot::error::TryRecvError::Empty) => None,
            _ => panic!("the sender was dropped without sending"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{pin_mut, FutureExt};
    use futures_timer::Delay;
    use std::time::Duration;
    use tokio::select;

    #[tokio::test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            if tx.send(3).is_err() {
                panic!("the receiver dropped");
            }
        });

        match rx.await {
            Ok(v) => assert_eq!(3, v),
            Err(_) => panic!("the sender dropped"),
        }
    }

    #[test]
    fn test_blocking_recv() {
        let task = AsyncTask::new(async move { 5 });
        assert_eq!(5, task.blocking_recv());
    }

    #[tokio::test]
    async fn test_try_recv() {
        let task = AsyncTask::new(async move { 5 });
        let (fut, mut rx) = task.into_parts();

        assert_eq!(None, rx.try_recv());

        // Spawn
        tokio::spawn(fut);

        // Wait for response
        let fetch = Delay::new(Duration::from_millis(1)).fuse();
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);
        select! {
            _ = &mut fetch => {
                if let Some(v) = rx.try_recv() {
                    assert_eq!(5, v);
                } else {
                    // Reset the clock
                    fetch.set(Delay::new(Duration::from_millis(1)).fuse());
                }
            }
            _ = timeout => panic!("timeout")
        };
    }
}
