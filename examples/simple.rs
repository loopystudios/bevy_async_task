//! Simple example - this demonstrates running one async task continuously.

use async_std::task::sleep;
use bevy::{app::PanicHandlerPlugin, log::LogPlugin, prelude::*};
use bevy_async_task::TaskRunner;
use std::{task::Poll, time::Duration};

/// An async task that takes time to compute!
async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_runner: TaskRunner<'_, u32>) {
    if task_runner.is_idle() {
        // Start an async task!
        task_runner.start(long_task());
        // Closures also work:
        // task_executor.start(async { 5 });
        info!("Started!");
    }

    match task_runner.poll() {
        Poll::Ready(v) => {
            info!("Received {v}");
        }
        Poll::Pending => {
            // Waiting...
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
