use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::AsyncTaskRunner;
use std::time::Duration;

async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_executor: AsyncTaskRunner<u32>) {
    let result = task_executor.blocking_recv(long_task());
    println!("Received {result}");
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_system(my_system)
        .run();
}
