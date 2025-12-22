use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use bevy_tasks::ConditionalSend;
use bevy_tasks::ConditionalSendFuture;
use bevy_tasks::futures_lite::Stream;
use bevy_tasks::futures_lite::StreamExt;
use bevy_tasks::futures_lite::stream;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::task::AtomicWaker;

use crate::ConditionalSendStream;
use crate::receiver::AsyncStreamReceiver;

/// An async stream task that yields multiple items.
pub struct AsyncStream<T: ConditionalSend> {
    stream: Pin<Box<dyn ConditionalSendStream<Item = T> + 'static>>,
}

impl<T: ConditionalSend> fmt::Debug for AsyncStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncStream")
            .field("stream", &"<stream>")
            .finish()
    }
}

impl<T: ConditionalSend + 'static> AsyncStream<T> {
    /// Create a new async stream from a future that produces a stream.
    pub fn lazy<F, S>(fut: F) -> Self
    where
        F: ConditionalSendFuture<Output = S> + 'static,
        S: ConditionalSendStream<Item = T> + 'static,
    {
        Self {
            stream: Box::pin(stream::once_future(fut).flatten()),
        }
    }

    /// Create a new async stream from a stream.
    pub fn new<S>(stream: S) -> Self
    where
        S: ConditionalSendStream<Item = T> + 'static,
        S::Item: ConditionalSend + 'static,
    {
        Self {
            stream: Box::pin(stream),
        }
    }

    /// Split the stream into a runnable future and receiver.
    /// This is a low-level operation and only useful for specific needs.
    pub fn split(
        self,
    ) -> (
        impl ConditionalSendFuture<Output = ()>,
        AsyncStreamReceiver<T>,
    ) {
        let (mut tx, rx) = mpsc::unbounded();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);
        let waker = Arc::new(AtomicWaker::new());
        let waker_clone = Arc::clone(&waker);
        let received = Arc::new(AtomicBool::new(false));
        let received_clone = Arc::clone(&received);

        let fut = async move {
            let mut stream = self.stream;

            while let Some(item) = stream.next().await {
                if tx.send(item).await.is_err() {
                    // Receiver dropped, exit early
                    break;
                }
            }
            finished_clone.store(true, Ordering::Relaxed);
            waker_clone.wake();

            // Wait for receiver to acknowledge it's done reading
            futures::future::poll_fn(|cx| {
                waker_clone.register(cx.waker());
                if received_clone.load(Ordering::Relaxed) {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
        };

        (
            fut,
            AsyncStreamReceiver {
                finished,
                waker,
                receiver: rx,
                received,
            },
        )
    }
}

impl<T, S> From<S> for AsyncStream<T>
where
    T: ConditionalSend + 'static,
    S: Stream<Item = T> + Send + 'static,
{
    fn from(stream: S) -> Self {
        Self::new(stream)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use core::time::Duration;

    use bevy_tasks::futures_lite::stream;
    use futures::FutureExt;
    use futures::pin_mut;
    use futures_timer::Delay;
    use tokio::select;

    use super::*;

    #[tokio::test]
    async fn test_stream_basic() {
        let items = vec![1, 2, 3, 4, 5];
        let stream = stream::iter(items.clone());
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        assert_eq!(None, rx.try_recv());
        assert!(!rx.is_finished());

        // Spawn
        tokio::spawn(fut);

        // Collect all items
        let mut collected = Vec::new();
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        collected.push(v);
                        if collected.len() == items.len() {
                            break 'result;
                        }
                        fetch.reset(Duration::from_millis(1));
                    } else if rx.is_finished() && collected.len() == items.len() {
                        break 'result;
                    } else {
                        fetch.reset(Duration::from_millis(1));
                    }
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert_eq!(items, collected);
        assert!(rx.is_finished());
    }

    #[tokio::test]
    async fn test_stream_empty() {
        let stream = stream::empty::<i32>();
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        assert_eq!(None, rx.try_recv());
        assert!(!rx.is_finished());

        // Spawn
        tokio::spawn(fut);

        // Wait for stream to finish
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if rx.is_finished() {
                        break 'result;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert_eq!(None, rx.try_recv());
        assert!(rx.is_finished());
    }

    #[tokio::test]
    async fn test_stream_single_item() {
        let stream = stream::once(42);
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Spawn
        tokio::spawn(fut);

        // Wait for item
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        assert_eq!(42, v);
                        break 'result;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        // Wait for finish
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'finish: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if rx.is_finished() {
                        break 'finish;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert!(rx.is_finished());
    }

    #[tokio::test]
    async fn test_stream_async_items() {
        // Create a stream that yields items with delays
        let stream = stream::iter(vec![1, 2, 3]).then(|x| async move {
            Delay::new(Duration::from_millis(5)).await;
            x * 2
        });

        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Spawn
        tokio::spawn(fut);

        // Collect all items
        let mut collected = Vec::new();
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(500)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        collected.push(v);
                        if collected.len() == 3 {
                            break 'result;
                        }
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert_eq!(vec![2, 4, 6], collected);
    }

    #[tokio::test]
    async fn test_stream_large_batch() {
        let items: Vec<i32> = (0..100).collect();
        let stream = stream::iter(items.clone());
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Spawn
        tokio::spawn(fut);

        // Collect all items
        let mut collected = Vec::new();
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(500)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    while let Some(v) = rx.try_recv() {
                        collected.push(v);
                    }
                    if rx.is_finished() && collected.len() == items.len() {
                        break 'result;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert_eq!(items, collected);
    }

    #[tokio::test]
    async fn test_stream_from_conversion() {
        let items = vec![1, 2, 3];
        let stream = stream::iter(items.clone());
        let task: AsyncStream<i32> = stream.into();
        let (fut, mut rx) = task.split();

        // Spawn
        tokio::spawn(fut);

        // Collect all items
        let mut collected = Vec::new();
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    while let Some(v) = rx.try_recv() {
                        collected.push(v);
                    }
                    if rx.is_finished() {
                        break 'result;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        assert_eq!(items, collected);
    }

    #[tokio::test]
    async fn test_stream_receiver_dropped_early() {
        let items = vec![1, 2, 3, 4, 5];
        let stream = stream::iter(items);
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Spawn
        let handle = tokio::spawn(fut);

        // Get first item then drop receiver
        let fetch = Delay::new(Duration::from_millis(1));
        let timeout = Delay::new(Duration::from_millis(100)).fuse();
        pin_mut!(timeout, fetch);

        'result: loop {
            select! {
                _ = (&mut fetch).fuse() => {
                    if let Some(v) = rx.try_recv() {
                        assert_eq!(1, v);
                        break 'result;
                    }
                    fetch.reset(Duration::from_millis(1));
                }
                _ = &mut timeout => panic!("timeout")
            };
        }

        // Drop receiver
        drop(rx);

        // Task should complete without panic
        let timeout = Delay::new(Duration::from_millis(100));
        tokio::select! {
            _ = handle => {},
            _ = timeout => panic!("task didn't complete after receiver dropped")
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod test {
    use bevy_tasks::futures_lite::stream;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::AsyncStream;

    #[wasm_bindgen_test]
    async fn test_stream_basic() {
        let items = vec![1, 2, 3, 4, 5];
        let stream = stream::iter(items.clone());
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        assert_eq!(None, rx.try_recv());
        assert!(!rx.is_finished());

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Collect all items
        let mut collected = Vec::new();
        while let Some(v) = rx.try_recv() {
            collected.push(v);
        }

        assert_eq!(items, collected);
        assert!(rx.is_finished());
    }

    #[wasm_bindgen_test]
    async fn test_stream_empty() {
        let stream = stream::empty::<i32>();
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        assert_eq!(None, rx.try_recv());
        assert!(!rx.is_finished());

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        assert_eq!(None, rx.try_recv());
        assert!(rx.is_finished());
    }

    #[wasm_bindgen_test]
    async fn test_stream_single_item() {
        let stream = stream::once(42);
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        let item = rx.try_recv().unwrap_or_else(|| {
            panic!("expected item after await");
        });
        assert_eq!(42, item);
        assert!(rx.is_finished());
    }

    #[wasm_bindgen_test]
    async fn test_stream_async_items() {
        use bevy_tasks::futures_lite::StreamExt;

        // Create a stream that yields items with delays
        let stream = stream::iter(vec![1, 2, 3]).then(|x| async move { x * 2 });

        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Collect all items
        let mut collected = Vec::new();
        while let Some(v) = rx.try_recv() {
            collected.push(v);
        }

        assert_eq!(vec![2, 4, 6], collected);
    }

    #[wasm_bindgen_test]
    async fn test_stream_large_batch() {
        let items: Vec<i32> = (0..100).collect();
        let stream = stream::iter(items.clone());
        let task = AsyncStream::new(stream);
        let (fut, mut rx) = task.split();

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Collect all items
        let mut collected = Vec::new();
        while let Some(v) = rx.try_recv() {
            collected.push(v);
        }

        assert_eq!(items, collected);
    }

    #[wasm_bindgen_test]
    async fn test_stream_from_conversion() {
        let items = vec![1, 2, 3];
        let stream = stream::iter(items.clone());
        let task: AsyncStream<i32> = stream.into();
        let (fut, mut rx) = task.split();

        // Convert to Promise and await it.
        JsFuture::from(wasm_bindgen_futures::future_to_promise(async move {
            fut.await;
            Ok(JsValue::NULL)
        }))
        .await
        .unwrap_or_else(|e| {
            panic!("awaiting promise failed: {e:?}");
        });

        // Collect all items
        let mut collected = Vec::new();
        while let Some(v) = rx.try_recv() {
            collected.push(v);
        }

        assert_eq!(items, collected);
    }
}
