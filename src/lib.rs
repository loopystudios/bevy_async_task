use bevy::ecs::system::{ReadOnlySystemParam, SystemMeta, SystemParam};
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::utils::synccell::SyncCell;
use futures::channel::oneshot;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::AsyncTask;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::AsyncTask;

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
        _world: &'w World,
        _change_tick: u32,
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
            Ok(Some(t)) => {
                self.received = true;
                self.buffer.close();
                Some(t)
            }
            Ok(None) => None,
            Err(_) => panic!("the sender was dropped without sending"),
        }
    }
}
