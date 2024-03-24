use crate::{AsyncReceiver, AsyncTask, AsyncTaskStatus};
use bevy::{
    ecs::{
        component::Tick,
        system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam},
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::synccell::SyncCell,
};

/// A Bevy [`SystemParam`] to execute [`AsyncTask`]s individually in the
/// background.
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

    /// Block awaiting the task result. Can only be used outside of async
    /// contexts.
    ///
    /// # Panics
    /// Panics if called within an async context.
    pub fn blocking_recv(&mut self, task: impl Into<AsyncTask<T>>) -> T {
        let task = task.into();
        task.blocking_recv()
    }

    /// Start an async task in the background. If there is an existing task
    /// pending, it will be dropped and replaced with the given task. If you
    /// need to run multiple tasks, use the [`AsyncTaskPool`].
    pub fn start(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.into_parts();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Poll the task runner for the current task status. If no task has begun,
    /// this will return `Idle`. Possible returns are `Idle`, `Pending`, or
    /// `Finished(T)`.
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

impl<'_s, T: Send + 'static> ExclusiveSystemParam for AsyncTaskRunner<'_s, T> {
    type State = SyncCell<Option<AsyncReceiver<T>>>;
    type Item<'s> = AsyncTaskRunner<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(None)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        AsyncTaskRunner(state.get())
    }
}

// SAFETY: only local state is accessed
unsafe impl<'s, T: Send + 'static> ReadOnlySystemParam for AsyncTaskRunner<'s, T> {}

// SAFETY: only local state is accessed
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
