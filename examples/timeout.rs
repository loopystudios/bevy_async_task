use async_std::{future::TimeoutError, task::sleep};
use bevy::prelude::*;
use bevy_async_task::{AsyncTaskRunner, AsyncTaskStatus, AsyncTimeoutTask};
use std::time::Duration;

async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_impatient_system(mut task_executor: AsyncTaskRunner<Result<u32, TimeoutError>>) {
    match task_executor.poll() {
        AsyncTaskStatus::Idle => {
            let timed_task = AsyncTimeoutTask::new(Duration::from_millis(500), long_task());
            task_executor.begin(timed_task);
            println!("Started!");
        }
        AsyncTaskStatus::Pending => {
            // Waiting...
        }
        AsyncTaskStatus::Finished(v) => match v {
            Ok(_) => panic!("This should always time-out"),
            Err(_) => println!("A timeout happened in `my_impatient_system`!"),
        },
    }
}

pub fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Main, my_impatient_system)
        .run();
}
