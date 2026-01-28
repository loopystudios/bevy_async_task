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

use crate::AsyncStream;
use crate::receiver::AsyncStreamReceiver;

/// A Bevy [`SystemParam`] to execute async streams in the background.
#[derive(Debug)]
pub struct TaskStream<'s, T>(pub(crate) &'s mut Option<AsyncStreamReceiver<T>>);

impl<T> TaskStream<'_, T>
where
    T: ConditionalSend + 'static,
{
    /// Returns whether the stream is idle (not started).
    pub fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    /// Returns whether the stream is pending (running, but not finished).
    pub fn is_pending(&self) -> bool {
        if let Some(rx) = &self.0 {
            !rx.is_finished()
        } else {
            false
        }
    }

    /// Returns whether the stream is finished.
    pub fn is_finished(&self) -> bool {
        if let Some(rx) = &self.0 {
            rx.is_finished()
        } else {
            false
        }
    }

    /// Start an async stream in the background. If there is an existing stream
    /// pending, it will be dropped and replaced with the given stream.
    pub fn start(&mut self, stream: impl Into<AsyncStream<T>>) {
        let stream = stream.into();
        let (fut, rx) = stream.split();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.replace(rx);
    }

    /// Forget the stream being run. This does not stop the stream.
    ///
    /// Note: Bevy does not support cancelling a task on web currently.
    pub fn forget(&mut self) {
        self.0.take();
    }

    /// Poll for the next item. Returns `Poll::Ready(Some(T))` if an item is available,
    /// `Poll::Ready(None)` if the stream is finished, or `Poll::Pending` if waiting.
    pub fn poll_next(&mut self) -> Poll<Option<T>> {
        match self.0.as_mut() {
            Some(rx) => match rx.try_recv() {
                Some(item) => Poll::Ready(Some(item)),
                None => {
                    if rx.is_finished() {
                        self.0.take();
                        Poll::Ready(None)
                    } else {
                        Poll::Pending
                    }
                }
            },
            None => Poll::Ready(None),
        }
    }
}

impl<T: Send + 'static> ExclusiveSystemParam for TaskStream<'_, T> {
    type State = SyncCell<Option<AsyncStreamReceiver<T>>>;
    type Item<'s> = TaskStream<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(None)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        TaskStream(state.get())
    }
}

// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> ReadOnlySystemParam for TaskStream<'_, T> {}

// SAFETY: only local state is accessed
unsafe impl<T: Send + 'static> SystemParam for TaskStream<'_, T> {
    type State = SyncCell<Option<AsyncStreamReceiver<T>>>;
    type Item<'w, 's> = TaskStream<'s, T>;

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
        TaskStream(state.get())
    }

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut bevy_ecs::query::FilteredAccessSet,
        _world: &mut World,
    ) {
    }
}
