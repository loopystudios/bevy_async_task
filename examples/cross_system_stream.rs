//! Cross system example - This example shows how to start a stream from one system and poll it from
//! another through a resource.
use std::task::Poll;
use std::time::Duration;

use bevy::app::PanicHandlerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy_async_task::AsyncStream;
use bevy_async_task::AsyncStreamReceiver;
use bevy_async_task::sleep;
use bevy_tasks::futures_lite::Stream;
use bevy_tasks::futures_lite::StreamExt;
use bevy_tasks::futures_lite::stream;

#[derive(Resource, DerefMut, Deref, Default)]
struct MyStream(Option<AsyncStreamReceiver<u32>>);

/// An async stream that yields numbers over time
async fn async_number_stream() -> impl Stream<Item = u32> {
    sleep(Duration::from_millis(500)).await;
    stream::iter(vec![1, 2, 3, 4, 5]).then(|x| async move {
        sleep(Duration::from_millis(500)).await;
        x * 10
    })
}

fn system1_start(mut my_stream: ResMut<'_, MyStream>) {
    let stream = AsyncStream::lazy(async_number_stream());
    let (fut, receiver) = stream.split();
    my_stream.replace(receiver);
    AsyncComputeTaskPool::get().spawn_local(fut).detach();
    info!("Stream started!");
}

fn system2_poll(mut my_stream: ResMut<'_, MyStream>, mut exit: MessageWriter<'_, AppExit>) {
    let Some(receiver) = my_stream.0.as_mut() else {
        return;
    };

    while let Some(v) = receiver.try_recv() {
        info!("Received {v}");
    }
    if receiver.is_finished() {
        info!("Stream finished!");
        exit.write(AppExit::Success);
    }
}

/// Entry point
pub fn main() {
    App::new()
        .init_resource::<MyStream>()
        .add_plugins((MinimalPlugins, LogPlugin::default(), PanicHandlerPlugin))
        .add_systems(Startup, system1_start)
        .add_systems(Update, system2_poll)
        .run();
}
