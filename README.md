# Bevy Async Task

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![crates.io](https://img.shields.io/crates/v/bevy-async-task.svg)](https://crates.io/crates/bevy-async-task)
[![docs.rs](https://img.shields.io/docsrs/bevy-async-task)](https://docs.rs/bevy-async-task)

This is a small crate that creates ergonomic abstractions to polling async compute tasks in Bevy.

Supports both **wasm** and **native**.

## Usage

Local polling for an async-task:

```rust
let task = AsyncTask::new(async move { 5 });
// Break the task into a runnable future and a receiver
let (fut, mut rx) = task.into_parts();
assert_eq!(None, rx.try_recv());
// Run the future
let task_pool = bevy::prelude::AsyncComputeTaskPool::get();
task_pool.spawn(fut);
// Receive the result
while let None = rx.try_recv() {}
assert_eq!(Some(5), rx.try_recv());
```

Polling an async-task:

```rust
let task = AsyncTask::new(async move { 5 });
// Break the task into a runnable future and a receiver
let (fut, mut rx) = task.into_parts();
assert_eq!(None, rx.try_recv());
// Run the future
let task_pool = bevy::prelude::AsyncComputeTaskPool::get();
let task = task_pool.spawn(fut);
task.detach(); // Run in background
// Wait for the result
let result = loop {
    if let Some(v) = rx.try_recv() {
        break v;
    }
};
assert_eq!(5, result);
```

Blocking an async-task:

```rust
use bevy_async_task::AsyncTask;

let task = AsyncTask::new(async move { 5 });
assert_eq!(5, task.blocking_recv());
```

## Bevy version support

|bevy|bevy_web_asset|
|---|---|
|main|main|
|0.10|0.1|

## License

This project is dual-licensed under both [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT) licenses.
