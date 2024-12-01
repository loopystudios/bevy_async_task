use crate::{AsyncReceiver, AsyncTask};
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
use std::task::Poll;

/// A Bevy [`SystemParam`] to execute many similar [`AsyncTask`]s in the
/// background simultaneously.
#[derive(Debug)]
pub struct AsyncTaskPool<'s, T>(pub(crate) &'s mut Vec<Option<AsyncReceiver<T>>>);

impl<T> AsyncTaskPool<'_, T> {
    /// Returns whether the task pool is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_empty() || !self.0.iter().any(Option::is_some)
    }

    /// Spawn an async task in the background.
    pub fn spawn(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.into_parts();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.push(Some(rx));
    }

    /// Iterate and poll the task pool for the current task statuses. A task can
    /// yield `Idle`, `Pending`, or `Finished(T)`.
    pub fn iter_poll(&mut self) -> impl Iterator<Item = Poll<T>> {
        let mut statuses = vec![];
        self.0.retain_mut(|task| match task {
            Some(rx) => {
                if let Some(v) = rx.try_recv() {
                    statuses.push(Poll::Ready(v));
                    false
                } else {
                    statuses.push(Poll::Pending);
                    true
                }
            }
            None => {
                statuses.push(Poll::Pending);
                true
            }
        });
        statuses.into_iter()
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for AsyncTaskPool<'_, T> {
    type State = SyncCell<Vec<Option<AsyncReceiver<T>>>>;
    type Item<'s> = AsyncTaskPool<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        AsyncTaskPool(state.get())
    }
}

// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for AsyncTaskPool<'_, T> {}

// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for AsyncTaskPool<'_, T> {
    type State = SyncCell<Vec<Option<AsyncReceiver<T>>>>;
    type Item<'w, 's> = AsyncTaskPool<'s, T>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        AsyncTaskPool(state.get())
    }
}
