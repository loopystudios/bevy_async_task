//! Timeout example - this demonstrates running one task with a timeout continuously.

use core::time::Duration;
use std::task::Poll;

use bevy::app::PanicHandlerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_async_task::TimedAsyncTask;
use bevy_async_task::TimedTaskRunner;
use bevy_async_task::TimeoutError;
use bevy_async_task::sleep;

fn system_does_timeout(mut task_executor: TimedTaskRunner<'_, ()>) {
    if task_executor.is_idle() {
        let timeout_task = TimedAsyncTask::pending().with_timeout(Duration::from_secs(2));
        task_executor.start(timeout_task);
        info!("Started A!");
    }

    match task_executor.poll() {
        Poll::Ready(Err(TimeoutError)) => {
            info!("Timeout on A! (expected)");
        }
        Poll::Pending => {
            // Waiting...
        }
        _ => unreachable!(),
    }
}

fn system_doesnt_timeout(mut task_executor: TimedTaskRunner<'_, u32>, mut counter: Local<'_, u32>) {
    if task_executor.is_idle() {
        let task = {
            *counter += 1;
            let next = *counter;
            async move {
                sleep(Duration::from_secs(2)).await;
                next
            }
        };
        let task = TimedAsyncTask::new(std::time::Duration::from_millis(2_147_483_647_u64), task);
        task_executor.start(task);
        info!("Started B!");
    }

    match task_executor.poll() {
        Poll::Ready(Ok(v)) => {
            info!("Received B: {v}!");
        }
        Poll::Pending => {
            // Waiting...
        }
        e => panic!("This shouldn't timeout!: {e:?}"),
    }
}

/// Entry point
pub fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), PanicHandlerPlugin))
        .add_systems(Update, system_doesnt_timeout)
        .add_systems(Update, system_does_timeout)
        .run();
}
