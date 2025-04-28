use crate::{AsyncReceiver, AsyncTask, TaskError, TimedAsyncTask};
use bevy::{
    ecs::{
        component::Tick,
        system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam},
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
    tasks::{AsyncComputeTaskPool, ConditionalSend},
    utils::synccell::SyncCell,
};
use std::task::Poll;

/// A Bevy [`SystemParam`] to execute many similar [`AsyncTask`]s in the
/// background simultaneously.
#[derive(Debug)]
pub struct TaskPool<'s, T>(pub(crate) &'s mut Vec<AsyncReceiver<T>>);

impl<T: ConditionalSend + 'static> TaskPool<'_, T> {
    /// Returns whether the task pool is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_empty()
    }

    /// Spawn an async task in the background.
    pub fn spawn(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.split();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.push(rx);
    }

    /// Iterate and poll the task pool for the current task statuses.
    pub fn iter_poll(&mut self) -> impl Iterator<Item = Poll<T>> {
        let mut statuses = vec![];
        self.0.retain_mut(|receiver| {
            if let Some(v) = receiver.try_recv() {
                statuses.push(Poll::Ready(v));
                false
            } else {
                statuses.push(Poll::Pending);
                true
            }
        });
        statuses.into_iter()
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for TaskPool<'_, T> {
    type State = SyncCell<Vec<AsyncReceiver<T>>>;
    type Item<'s> = TaskPool<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        TaskPool(state.get())
    }
}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for TaskPool<'_, T> {}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for TaskPool<'_, T> {
    type State = SyncCell<Vec<AsyncReceiver<T>>>;
    type Item<'w, 's> = TaskPool<'s, T>;

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
        TaskPool(state.get())
    }
}

/// A Bevy [`SystemParam`] to execute many similar [`AsyncTask`]s in the
/// background simultaneously.
#[derive(Debug)]
pub struct TimedTaskPool<'s, T>(pub(crate) &'s mut Vec<AsyncReceiver<Result<T, TaskError>>>);

impl<T: ConditionalSend + 'static> TimedTaskPool<'_, T> {
    /// Returns whether the task pool is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_empty()
    }

    /// Spawn an async task in the background.
    pub fn spawn(&mut self, task: impl Into<TimedAsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.split();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.push(rx);
    }

    /// Iterate and poll the task pool for the current task statuses.
    pub fn iter_poll(&mut self) -> impl Iterator<Item = Poll<Result<T, TaskError>>> {
        let mut statuses = vec![];
        self.0.retain_mut(|receiver| {
            if let Some(v) = receiver.try_recv() {
                statuses.push(Poll::Ready(v));
                false
            } else {
                statuses.push(Poll::Pending);
                true
            }
        });
        statuses.into_iter()
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for TimedTaskPool<'_, T> {
    type State = SyncCell<Vec<AsyncReceiver<Result<T, TaskError>>>>;
    type Item<'s> = TimedTaskPool<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        TimedTaskPool(state.get())
    }
}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for TimedTaskPool<'_, T> {}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for TimedTaskPool<'_, T> {
    type State = SyncCell<Vec<AsyncReceiver<Result<T, TaskError>>>>;
    type Item<'w, 's> = TimedTaskPool<'s, T>;

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
        TimedTaskPool(state.get())
    }
}
