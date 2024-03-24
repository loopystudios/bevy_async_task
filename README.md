# Bevy Async Task

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![crates.io](https://img.shields.io/crates/v/bevy_async_task.svg)](https://crates.io/crates/bevy_async_task)
[![docs.rs](https://img.shields.io/docsrs/bevy_async_task)](https://docs.rs/bevy_async_task)

A minimum crate for ergonomic abstractions to async programming in Bevy for all platforms. This crate helps to run async tasks in the background with timeout support and retrieve results in the same system, and helps to block on futures within synchronous contexts.

There is full API support for **wasm** and **native**. Android and iOS are untested (Help needed).

## Bevy version support

|bevy|bevy_async_task|
|---|---|
|0.13|0.1, main|
|<= 0.13|Unsupported|

## Usage

Please see [examples](examples/) for more.

### Polling in systems

Poll one task at a time with `AsyncTaskRunner<T>`:

```rust
async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_executor: AsyncTaskRunner<u32>) {
    match task_executor.poll() {
        AsnycTaskStatus::Idle => {
            task_executor.begin(long_task());
            println!("Started new task!");
        }
        AsnycTaskStatus::Pending => {
            // <Insert loading screen>
        }
        AsnycTaskStatus::Finished(v) => {
            println!("Received {v}");
        }
    }
}
```

Poll many similar tasks simultaneously with `AsyncTaskPool<T>`:

```rust
fn my_system(mut task_pool: AsyncTaskPool<u64>) {
    if task_pool.is_idle() {
        println!("Queueing 5 tasks...");
        for i in 1..=5 {
            task_pool.spawn(async move { // Closures work too!
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
```

Also, you may use timeouts or block on an `AsyncTask<T>`:

```rust
// Blocking:
let task = AsyncTask::new(async { 5 });
assert_eq!(5, task.blocking_recv());

// Timeout:
let task = AsyncTask::<()>::pending().with_timeout(Duration::from_millis(10));
assert!(task.blocking_recv().is_err());
```

Need to steer manually? Break the task into parts.

```rust
let task = AsyncTask::new(async move {
    sleep(Duration::from_millis(1000)).await;
    5
});
// Break the task into a runnable future and a receiver
let (fut, mut rx) = task.into_parts();
// The receiver will always be `None` until it is polled by Bevy.
assert_eq!(None, rx.try_recv());
// Run the future
let task_pool = bevy::prelude::AsyncComputeTaskPool::get();
let task = task_pool.spawn(fut);
task.detach(); // Forget and run in background
// Spin-lock, waiting for the result
let result = loop {
    if let Some(v) = rx.try_recv() {
        break v;
    }
};
assert_eq!(5, result);
```

## Community

All Loopy projects and development happens in the [Loopy Discord](https://discord.gg/zrjnQzdjCB). The discord is open to the public.

Contributions are welcome by pull request. The [Rust code of conduct](https://www.rust-lang.org/policies/code-of-conduct) applies.

## License

Licensed under either of

- Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
