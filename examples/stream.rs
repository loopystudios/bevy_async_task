//! Simple example - this demonstrates running one async stream continuously.

use std::task::Poll;
use std::time::Duration;

use bevy::app::PanicHandlerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_async_task::AsyncStream;
use bevy_async_task::TaskStream;
use bevy_async_task::sleep;
use bevy_tasks::futures_lite::Stream;
use bevy_tasks::futures_lite::StreamExt;
use bevy_tasks::futures_lite::stream;

/// An async stream that yields numbers over time
async fn async_number_stream() -> impl Stream<Item = u32> {
    sleep(Duration::from_millis(500)).await;

    stream::iter(vec![1, 2, 3, 4, 5]).then(|x| async move {
        sleep(Duration::from_millis(500)).await;
        x * 10
    })
}

fn my_system(mut task_stream: TaskStream<'_, u32>) {
    if task_stream.is_idle() {
        // Start an async stream!
        let stream = AsyncStream::lazy(async_number_stream());
        task_stream.start(stream);
        info!("Stream started!");
    }

    match task_stream.poll_next() {
        Poll::Ready(Some(v)) => {
            info!("Received {v}");
        }
        Poll::Ready(None) => {
            info!("Stream finished!");
        }
        Poll::Pending => {
            // Waiting for next item...
        }
    }
}

/// Entry point
pub fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), PanicHandlerPlugin))
        .add_systems(Update, my_system)
        .run();
}
