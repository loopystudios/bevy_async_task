# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- This release supports **Bevy 0.17**.

## [0.11] - 2025-12-12

- This release supports **Bevy 0.17**.

### Added

- Added `AsyncStream` and `TaskStream`, functional equivalents of `AsyncTask` and `TaskRunner` for streams (aka async iterators). See the streaming example.
- Added `.forget()` to `TaskRunner` and `TimedTaskRunner`.
- Added `.forget_all()` to `TaskPool` and `TimedTaskPool`.

### Removed

- Removed `Deref` from `TaskRunner` and `TimedTaskRunner`.
- Removed `DerefMut` from `TaskRunner` and `TimedTaskRunner`.

### Fixed

- It was possible to use a duration over `i32::MAX` in millis for `AsyncTask::with_duration(&mut self)`.

## [0.10.0] - 2025-11-28

- This release supports **Bevy 0.17**.

### Added

- Added `AsyncStream` and `TaskStream`, functional equivalents of `AsyncTask` and `TaskRunner` for streams (aka async iterators). See the streaming example.
- Added `bevy_async_task::MAX_TIMEOUT`
- Added `bevy_async_task::DEFAULT_TIMEOUT`

### Changed

- Removed `bevy_async_task::Duration`. Use `core::time::Duration` instead.
- The default timeout for timed tasks, was changed from `u16::MAX` millis (~65 seconds) to 60 seconds.
- The maximum timeout allowed is now `i32::MAX`. This is now enforced with a panic.

### Fixed

- A `time not implemented on this platform` panic due to `wasm-bindgen` feature not being included for the `futures-timer` crate. You can remedy this yourself for older versions of `bevy_async_task` by including `futures-timer = { version = "3.0.3", features = ["wasm-bindgen"] }` in your Cargo.toml file.

## [0.9.0] - 2025-10-09

- This release supports **Bevy 0.17**.

### Changed

- Migrated to **Bevy 0.17**.

## [0.8.1] - 2025-07-14

### Fixed

- `bevy_async_task` no longer depends on the entire `bevy` crate — only the relevant subcrates (e.g., `bevy_ecs`).

## [0.8.0] - 2025-06-10

### Added

- Exposed utility functions: `timeout`, `sleep`, and `pending`.
- Added `AsyncTask::sleep`.

## [0.7.0] - 2025-05-03

### Changed

- Replaced the `TaskError` enum with a `TimeoutError` struct.

### Fixed

- Fixed a panic on WebAssembly (WASM) when timeouts exceeded numerical bounds.

## [0.6.0] - 2025-03-17

### Changed

- Updated to **Bevy 0.16**.

## [0.5.0] - 2024-12-22

### Changed

- Updated to **Rust 2024 Edition**.
- Split `AsyncTaskRunner` into `TimedTaskRunner` and `TaskRunner`. The timed variant polls timeouts.
- Split `AsyncTaskPool` into `TimedTaskPool` and `TaskPool`. The timed variant polls timeouts.
- Renamed `AsyncTask::build` to `AsyncTask::split`.

## [0.4.1] - 2024-10-11

### Fixed

- Removed repetitive console warnings about polling tasks.

## [0.4.0] - 2024-09-30

### Added

- Reintroduced `AsyncTask::with_timeout()`.

### Changed

- `poll()` and `iter_poll()` now return `Result<T, TimeoutError>`.
- Replaced `std::time::Duration` with `web_time::Duration`.
- Replaced `into_parts()` with `build()`.

## [0.3.0] - 2024-06-09

### Added

- Implemented `Debug` for many types.

### Changed

- Updated to **Bevy 0.15**.
- Replaced `AsyncTaskStatus` with `std::task::Poll`.
  - To check if a task runner or pool is idle, use `<AsyncTaskRunner>::is_idle()` or `<AsyncTaskPool>::is_idle()`.
- Made `TimeoutError` non-exhaustive for forward compatibility.

### Removed

- Removed `blocking_recv()` functions. Use `bevy::tasks::block_on(fut)` instead.
- Temporarily removed `AsyncTask::with_timeout()` pending rework. Use `AsyncTask::new_with_duration(duration, f)` as an alternative.

### Fixed

- Timeouts now function correctly on `wasm32`.
- `AsyncReceiver` now uses an `AtomicWaker` to ensure the sender isn’t dropped prematurely.

## [0.2.0] - 2024-03-14

### Changed

- Updated to **Bevy 0.14**.

### Fixed

- Fixed blocking example for web targets.

## [0.1.1] - 2024-02-09

### Fixed

- Re-uploaded README to correct [#10](https://github.com/loopystudios/bevy_async_task/issues/10).

## [0.1.0] - 2024-02-01

### Added

- Initial release.
- Crate renamed from `bevy-async-task` to `bevy_async_task` and republished.

---

[unreleased]: https://github.com/loopystudios/bevy_async_task/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/loopystudios/bevy_async_task/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/loopystudios/bevy_async_task/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/loopystudios/bevy_async_task/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/loopystudios/bevy_async_task/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/loopystudios/bevy_async_task/releases/tag/v0.1.0
