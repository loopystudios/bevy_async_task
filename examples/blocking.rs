use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::{AsyncTask, AsyncTaskRunner};
use std::time::Duration;

fn system1(mut task_executor: AsyncTaskRunner<u32>) {
    let result = task_executor.blocking_recv(async {
        sleep(Duration::from_millis(1000)).await;
        1
    });
    println!("Received {result}");
}

fn system2() {
    let result = AsyncTask::new(async {
        sleep(Duration::from_millis(1000)).await;
        2
    })
    .blocking_recv();
    println!("Received {result}");
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_system(system1)
        .add_system(system2)
        .run();
}
