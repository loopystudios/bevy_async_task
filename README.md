<div align="center">

# Bevy Async Task

[![Discord](https://img.shields.io/discord/913957940560531456.svg?label=Loopy&logo=discord&logoColor=ffffff&color=ffffff&labelColor=000000)](https://discord.gg/zrjnQzdjCB)
![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Build status](https://github.com/loopystudios/bevy_async_task/workflows/CI/badge.svg)](https://github.com/loopystudios/bevy_async_task/actions)
[![Dependency status](https://deps.rs/repo/github/loopystudios/bevy_async_task/status.svg)](https://deps.rs/repo/github/loopystudios/bevy_async_task)
[![crates.io](https://img.shields.io/crates/v/bevy_async_task.svg)](https://crates.io/crates/bevy_async_task)
[![docs.rs](https://img.shields.io/docsrs/bevy_async_task)](https://docs.rs/bevy_async_task)

A minimum crate for ergonomic abstractions to async programming in Bevy. There is full API support for **wasm** and **native**. Android and iOS are untested (Help needed).

</div>

Bevy Async Task provides Bevy system parameters to run asynchronous tasks in the background on web and native with timeouts and output capture.

## Bevy version support

|bevy|bevy_async_task|
|---|---|
|0.15|0.3-0.4, main|
|0.14|0.2|
|0.13|0.1|
|<= 0.13|Unsupported|

## Usage

There are several [examples](examples/) for reference.

You can also run examples on web:

```shell
# Make sure the Rust toolchain supports the wasm32 target
rustup target add wasm32-unknown-unknown

cargo run_wasm --example simple
```

### Polling in systems

Poll one task at a time with `AsyncTaskRunner<T>`:

```rust
async fn long_task() -> u32 {
    sleep(Duration::from_millis(1000)).await;
    5
}

fn my_system(mut task_runner: AsyncTaskRunner<u32>) {
    if task_runner.is_idle() {
        task_executor.start(long_task());
        info!("Started!");
    }

    match task_runner.poll() {
        Poll::Ready(v) => {
            info!("Received {v:?}");
        }
        Poll::Pending => {
            // Waiting...
        }
    }
}
```

Poll many similar tasks simultaneously with `AsyncTaskPool<T>`:

```rust
fn my_system(mut task_pool: AsyncTaskPool<u64>) {
    if task_pool.is_idle() {
        info!("Queueing 5 tasks...");
        for i in 1..=5 {
            task_pool.spawn(async move { // Closures work too!
                sleep(Duration::from_millis(i * 1000)).await;
                i
            });
        }
    }

    for status in task_pool.iter_poll() {
        if let Poll::Ready(v) = status {
            info!("Received {v:?}");
        }
    }
}
```

We also have a [`cross_system` example](./examples/cross_system.rs).

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
