//! Timeout example - this demonstrates running one task with a timeout continuously.

use async_std::task::sleep;
use bevy::{app::PanicHandlerPlugin, log::LogPlugin, prelude::*};
use bevy_async_task::{Duration, TimedAsyncTask, TimedTaskRunner, TimeoutError};
use std::task::Poll;

fn system_does_timeout(mut task_executor: TimedTaskRunner<'_, ()>) {
    if task_executor.is_idle() {
        let timeout_task = TimedAsyncTask::pending().with_timeout(Duration::from_secs(2));
        task_executor.start(timeout_task);
        info!("Started A!");
    }

    match task_executor.poll() {
        Poll::Ready(Err(TimeoutError)) => {
            info!("Timeout on A!");
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
        e => panic!("{e:?}"),
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
