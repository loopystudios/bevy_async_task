use crate::{AsyncReceiver, AsyncTask, AsyncTaskStatus};
use bevy::{
    ecs::system::{ReadOnlySystemParam, SystemMeta, SystemParam},
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::synccell::SyncCell,
};

/// A task pool which executes [`AsyncTask`]s in the background.
pub struct AsyncTaskPool<'s, T>(pub(crate) &'s mut Vec<Option<AsyncReceiver<T>>>);

impl<'s, T> AsyncTaskPool<'s, T> {
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

    /// Iterate and poll the task pool for the current task statuses. A task can yield `Idle`, `Pending`, or `Finished(T)`.
    pub fn iter_poll(&mut self) -> impl Iterator<Item = AsyncTaskStatus<T>> {
        let mut statuses = vec![];
        self.0.retain_mut(|task| match task {
            Some(rx) => match rx.try_recv() {
                Some(v) => {
                    statuses.push(AsyncTaskStatus::Finished(v));
                    false
                }
                None => {
                    statuses.push(AsyncTaskStatus::Pending);
                    true
                }
            },
            None => {
                statuses.push(AsyncTaskStatus::Idle);
                true
            }
        });
        statuses.into_iter()
    }
}

// SAFETY: Only accesses internal state locally, similar to bevy's `Local`
unsafe impl<'s, T: Send + 'static> ReadOnlySystemParam for AsyncTaskPool<'s, T> {}

// SAFETY: Only accesses internal state locally, similar to bevy's `Local`
unsafe impl<'a, T: Send + 'static> SystemParam for AsyncTaskPool<'a, T> {
    type State = SyncCell<Vec<Option<AsyncReceiver<T>>>>;
    type Item<'w, 's> = AsyncTaskPool<'s, T>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        AsyncTaskPool(state.get())
    }
}
