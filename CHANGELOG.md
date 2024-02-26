# Changelog

This changelog follows the patterns described here: <https://keepachangelog.com/en/1.0.0/>.

Subheadings to categorize changes are `added, changed, deprecated, removed, fixed, security`.

## 1.4.0

### changed

Updated to bevy 0.13

## 1.3.1

## added

- `TaskRunner` and `TaskPool` now can be exclusive systems in Bevy

## changed

- internally, oneshot channels are now tokio-based
- `blocking_recv` now uses `bevy_task`, and panics are are no longer possible

## fixed

- native oneshot channels dropping due to the missing `multi-threaded` feature on bevy 0.12

## 1.3.0

### changed

- changed to bevy 0.12


## 1.2.0

### added

- exported `TimeoutError`

## 1.1.0

### added

- added `AsyncTask::with_timeout(mut self, dur)`
- added timeout support via `AsyncTask::new_with_timeout(dur, f)`
- added `AsyncTask::pending()` as an abstraction for a task that never finishes

## 1.0.1

### fixed

- broken documentation links are fixed

## 1.0.0

- Initialize release.
