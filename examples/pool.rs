use async_std::task::sleep;
use bevy::prelude::*;
use bevy_async_task::{AsyncTaskPool, AsyncTaskStatus};
use std::time::Duration;

fn system1(mut task_pool: AsyncTaskPool<u64>) {
    if task_pool.is_idle() {
        println!("Queueing concurrent tasks...");
        for i in 1..=5 {
            task_pool.spawn(async move {
                sleep(Duration::from_millis(i * 1000)).await;
                i
            });
        }
    }

    for status in task_pool.iter_poll() {
        if let AsyncTaskStatus::Finished(t) = status {
            println!("Received {t}");
        }
    }
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_system(system1)
        .run();
}
