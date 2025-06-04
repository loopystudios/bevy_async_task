# Changelog

<!-- Instructions

This changelog follows the patterns described here: <https://keepachangelog.com/en/1.0.0/>.

Subheadings to categorize changes are `added, changed, deprecated, removed, fixed, security`.

-->

## Unreleased

- Nothing yet!

## 0.8.1

### Fixed

- `bevy_async_task` no longer uses the entire bevy dependency, just the relevant crates directly (e.g. `bevy_ecs`).

## 0.8.0

### Added

- Exposed `timeout`, `sleep`, and `pending` utility functions.
- Added `AsyncTask::sleep`

## 0.7.0

### Changed

- The `TaskError` enum has been replaced with a `TimeoutError` struct.

### Fixed

- A panic was fixed on WASM when the timeout exceeded numerical bounds.

## 0.6.0

### Changed

- Updated to Bevy 0.16.

## 0.5.0

### Changed

- Updated to rust 2024 edition.
- Split `AsyncTaskRunner` into `TimedTaskRunner` and `TaskRunner`. The timed variant polls timeouts.
- Split `AsyncTaskPool` into `TimedTaskPool` and `TaskPool`. The timed variant polls timeouts.
- `AsyncTask::build` has been renamed to `AsyncTask::split`.

## 0.4.1

### Fixed

- There is no longer repetitive console warnings about polling tasks.

## 0.4.0

### Added

- `AsyncTask.with_timeout()` has been added back.

### Changed

- `poll()` and `iter_poll()` now return a `Result<T, TimeoutError>`.
- `std::time::Duration` has been replaced with `web_time::Duration`.
- `into_parts()` has been replaced with `build()`.

## 0.3.0

### Added

- Many types now implement `Debug`.

### Changed

- Updated to bevy 0.15.
- `AsyncTaskStatus` was replaced with `std::task::Poll`. If you wish to check if a task runner or pool is idle, you can still do so with `<AsyncTaskRunner>.is_idle()`/`<AsyncTaskPool>.is_idle()`.
- Replaced `TimeoutError` with a non-exhaustive `TimeoutError` for future proofing.

### Removed

- `blocking_recv()` functions were removed. You should now use `bevy::tasks::block_on(fut)`.
- `AsyncTask.with_timeout()` until further rework is done to return this functionality. Please use `AsyncTask::new_with_duration(Dur, F)` instead.

### Fixed

- Timeouts now work correctly on wasm32.
- `AsyncReceiver` now uses an `AtomicWaker` to ensure the sender is never dropped before receiving.

## 0.2.0

### Changed

- Updated to bevy 0.14

### Fixed

- Blocking example for web targets.

## 0.1.1

### Fixed

- READMDE re-uploaded to correct [#10](https://github.com/loopystudios/bevy_async_task/issues/10).

## 0.1.0

- Initial release. Crate was renamed to `bevy_async_task` from `bevy-async-task` and republished.
