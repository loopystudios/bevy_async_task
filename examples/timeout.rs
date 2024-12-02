//! Timeout example - this demonstrates running one task with a timeout continuously.

use async_std::{future::pending, task::sleep};
use bevy::{app::PanicHandlerPlugin, log::LogPlugin, prelude::*};
use bevy_async_task::{AsyncTask, AsyncTaskRunner, Duration, TaskError};
use std::task::Poll;

fn system_does_timeout(mut task_executor: AsyncTaskRunner<'_, ()>) {
    if task_executor.is_idle() {
        let timeout_task = AsyncTask::new_with_timeout(Duration::from_secs(1), pending());
        task_executor.start(timeout_task);
        info!("Started A!");
    }

    match task_executor.poll() {
        Poll::Ready(Err(TaskError::Timeout(_))) => {
            info!("Timeout on A!");
        }
        Poll::Pending => {
            // Waiting...
        }
        _ => unreachable!(),
    }
}

fn system_doesnt_timeout(mut task_executor: AsyncTaskRunner<'_, u32>) {
    if task_executor.is_idle() {
        let timeout_task = AsyncTask::new_with_timeout(Duration::from_secs(10), async {
            sleep(Duration::from_secs(2)).await;
            5
        });
        task_executor.start(timeout_task);
        info!("Started B!");
    }

    match task_executor.poll() {
        Poll::Ready(Ok(v)) => {
            info!("Received B: {v}!");
        }
        Poll::Pending => {
            // Waiting...
        }
        _ => unreachable!(),
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
