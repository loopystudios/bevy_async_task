use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::{AsyncTaskRunner, AsyncTaskStatus};
use std::time::Duration;

async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_executor: AsyncTaskRunner<u32>) {
    match task_executor.poll() {
        AsyncTaskStatus::Idle => {
            task_executor.start(long_task());
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
