use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::{AsnycTaskStatus, AsyncTaskRunner};
use std::time::Duration;

async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_executor: AsyncTaskRunner<u32>) {
    match task_executor.poll() {
        AsnycTaskStatus::Idle => {
            task_executor.begin(long_task());
            println!("Started!");
        }
        AsnycTaskStatus::Pending => {
            // Waiting...
        }
        AsnycTaskStatus::Finished(v) => {
            println!("Received {v}");
        }
    }
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_system(my_system)
        .run();
}
