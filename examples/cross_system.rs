//! Cross system example - This example shows how to start a task from one system and poll it from
//! another through a resource.

use bevy::{app::PanicHandlerPlugin, log::LogPlugin, prelude::*, tasks::AsyncComputeTaskPool};
use bevy_async_task::{AsyncReceiver, AsyncTask, Duration, sleep};

#[derive(Resource, DerefMut, Deref, Default)]
struct MyTask(Option<AsyncReceiver<u32>>);

/// An async task that takes time to compute!
async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn system1_start(mut my_task: ResMut<'_, MyTask>) {
    let (fut, receiver) = AsyncTask::new(long_task()).split();
    my_task.replace(receiver);
    AsyncComputeTaskPool::get().spawn_local(fut).detach();
    info!("Started!");
}

fn system2_poll(mut my_task: ResMut<'_, MyTask>, mut exit: MessageWriter<'_, AppExit>) {
    let Some(receiver) = my_task.0.as_mut() else {
        return;
    };
    match receiver.try_recv() {
        Some(v) => {
            info!("Received {v}");
            exit.write(AppExit::Success);
        }
        None => {
            // Waiting...
        }
    }
}

/// Entry point
pub fn main() {
    App::new()
        .init_resource::<MyTask>()
        .add_plugins((MinimalPlugins, LogPlugin::default(), PanicHandlerPlugin))
        .add_systems(Startup, system1_start)
        .add_systems(Update, system2_poll)
        .run();
}
