use bevy::{app::PanicHandlerPlugin, log::LogPlugin, prelude::*};
use bevy_async_task::AsyncTask;
use std::time::Duration;

/// Use a timeout
fn system() {
    let _timeout = AsyncTask::<()>::pending()
        .with_timeout(Duration::from_millis(1000))
        .blocking_recv()
        .unwrap_err();

    info!("Timeout!");
}

pub fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), PanicHandlerPlugin))
        .add_systems(Update, system)
        .run();
}
