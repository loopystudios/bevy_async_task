use crate::{AsyncReceiver, AsyncTask, TaskError};
use bevy::{
    ecs::{
        component::Tick,
        system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam},
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::{synccell::SyncCell, ConditionalSend},
};
use std::{sync::atomic::Ordering, task::Poll};

/// A Bevy [`SystemParam`] to execute [`AsyncTask`]s individually in the
/// background.
#[derive(Debug)]
pub struct AsyncTaskRunner<'s, T>(pub(crate) &'s mut Option<AsyncReceiver<T>>);

impl<T: ConditionalSend + 'static> AsyncTaskRunner<'_, T> {
    /// Returns whether the task runner is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    /// Returns whether the task runner is pending (running, but not finished).
    pub fn is_pending(&self) -> bool {
        if let Some(ref rx) = self.0 {
            !rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Returns whether the task runner is finished.
    pub fn is_finished(&self) -> bool {
        if let Some(ref rx) = self.0 {
            rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Start an async task in the background. If there is an existing task
    /// pending, it will be dropped and replaced with the given task. If you
    /// need to run multiple tasks, use the [`AsyncTaskPool`].
    pub fn start(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.build();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Poll the task runner for the current task status. Possible returns are `Pending` or
    /// `Ready(T)`.
    pub fn poll(&mut self) -> Poll<Result<T, TaskError>> {
        match self.0.as_mut() {
            Some(rx) => match rx.try_recv() {
                Some(v) => {
                    self.0.take();
                    Poll::Ready(v)
                }
                None => Poll::Pending,
            },
            None => {
                warn!("You are polling a task runner before a task was started");
                Poll::Pending
            }
        }
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for AsyncTaskRunner<'_, T> {
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
unsafe impl<T: Send + 'static> ReadOnlySystemParam for AsyncTaskRunner<'_, T> {}

// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for AsyncTaskRunner<'_, T> {
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
