use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::{AsyncTaskRunner, AsyncTaskStatus};
use std::time::Duration;

/// An async task that takes time to compute!
async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_executor: AsyncTaskRunner<u32>) {
    match task_executor.poll() {
        AsyncTaskStatus::Idle => {
            // Start an async task!
            task_executor.start(long_task());
            // Closures also work:
            // task_executor.start(async { 5 });
            println!("Started!");
        }
        AsyncTaskStatus::Pending => {
            // Waiting...
        }
        AsyncTaskStatus::Finished(v) => {
            println!("Received {v}");
        }
    }
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Update, my_system)
        .run();
}
