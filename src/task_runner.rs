use std::sync::atomic::Ordering;
use std::task::Poll;

use bevy_ecs::change_detection::Tick;
use bevy_ecs::system::ExclusiveSystemParam;
use bevy_ecs::system::ReadOnlySystemParam;
use bevy_ecs::system::SystemMeta;
use bevy_ecs::system::SystemParam;
use bevy_ecs::world::World;
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_platform::cell::SyncCell;
use bevy_tasks::AsyncComputeTaskPool;
use bevy_tasks::ConditionalSend;

use crate::AsyncReceiver;
use crate::AsyncTask;
use crate::TimedAsyncTask;
use crate::TimeoutError;

/// A Bevy [`SystemParam`] to execute async tasks in the background.
#[derive(Debug)]
pub struct TaskRunner<'s, T>(pub(crate) &'s mut Option<AsyncReceiver<T>>);

impl<T> TaskRunner<'_, T>
where
    T: ConditionalSend + 'static,
{
    /// Returns whether the task runner is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    /// Returns whether the task runner is pending (running, but not finished).
    pub fn is_pending(&self) -> bool {
        if let Some(rx) = &self.0 {
            !rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Returns whether the task runner is finished.
    pub fn is_finished(&self) -> bool {
        if let Some(rx) = &self.0 {
            rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Start an async task in the background. If there is an existing task
    /// pending, it will be dropped and replaced with the given task. If you
    /// need to run multiple tasks, use the [`TaskPool`](crate::TaskPool).
    pub fn start(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.split();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Forget the task being run. This does not stop the task.
    ///
    /// Note: Bevy does not support cancelling a task on web currently.
    pub fn forget(&mut self) {
        self.0.take();
    }

    /// Poll the task runner for the current task status. Possible returns are `Pending` or
    /// `Ready(T)`.
    pub fn poll(&mut self) -> Poll<T> {
        match self.0.as_mut() {
            Some(rx) => match rx.try_recv() {
                Some(v) => {
                    self.0.take();
                    Poll::Ready(v)
                }
                None => Poll::Pending,
            },
            None => Poll::Pending,
        }
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for TaskRunner<'_, T> {
    type State = SyncCell<Option<AsyncReceiver<T>>>;
    type Item<'s> = TaskRunner<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(None)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        TaskRunner(state.get())
    }
}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for TaskRunner<'_, T> {}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for TaskRunner<'_, T> {
    type State = SyncCell<Option<AsyncReceiver<T>>>;
    type Item<'w, 's> = TaskRunner<'s, T>;

    fn init_state(_world: &mut World) -> Self::State {
        SyncCell::new(None)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        TaskRunner(state.get())
    }

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut bevy_ecs::query::FilteredAccessSet,
        _world: &mut World,
    ) {
    }
}

/// A Bevy [`SystemParam`] to execute async tasks in the background with a timeout.
#[derive(Debug)]
pub struct TimedTaskRunner<'s, T>(
    pub(crate) &'s mut Option<AsyncReceiver<Result<T, TimeoutError>>>,
);

impl<T> TimedTaskRunner<'_, T>
where
    T: ConditionalSend + 'static,
{
    /// Returns whether the task runner is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    /// Returns whether the task runner is pending (running, but not finished).
    pub fn is_pending(&self) -> bool {
        if let Some(rx) = &self.0 {
            !rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Returns whether the task runner is finished.
    pub fn is_finished(&self) -> bool {
        if let Some(rx) = &self.0 {
            rx.received.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    /// Start an async task in the background. If there is an existing task
    /// pending, it will be dropped and replaced with the given task. If you
    /// need to run multiple tasks, use the [`TimedTaskPool`](crate::TimedTaskPool).
    pub fn start(&mut self, task: impl Into<TimedAsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.split();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Forget the task being run. This does not stop the task.
    ///
    /// Note: Bevy does not support cancelling a task on web currently.
    pub fn forget(&mut self) {
        self.0.take();
    }

    /// Poll the task runner for the current task status. Possible returns are `Pending` or
    /// `Ready(T)`.
    pub fn poll(&mut self) -> Poll<Result<T, TimeoutError>> {
        match self.0.as_mut() {
            Some(rx) => match rx.try_recv() {
                Some(v) => {
                    self.0.take();
                    Poll::Ready(v)
                }
                None => Poll::Pending,
            },
            None => Poll::Pending,
        }
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for TimedTaskRunner<'_, T> {
    type State = SyncCell<Option<AsyncReceiver<Result<T, TimeoutError>>>>;
    type Item<'s> = TimedTaskRunner<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(None)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        TimedTaskRunner(state.get())
    }
}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for TimedTaskRunner<'_, T> {}
// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for TimedTaskRunner<'_, T> {
    type State = SyncCell<Option<AsyncReceiver<Result<T, TimeoutError>>>>;
    type Item<'w, 's> = TimedTaskRunner<'s, T>;

    fn init_state(_world: &mut World) -> Self::State {
        SyncCell::new(None)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        TimedTaskRunner(state.get())
    }

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut bevy_ecs::query::FilteredAccessSet,
        _world: &mut World,
    ) {
    }
}
