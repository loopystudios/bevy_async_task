/// A channel that catches an [`AsyncTask`](crate::AsyncTask) result.
pub struct AsyncReceiver<T> {
    pub(crate) received: bool,
    pub(crate) buffer: futures::channel::oneshot::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Poll the current thread waiting for the async result.
    ///
    /// # Panics
    /// Panics if the sender was dropped without sending
    pub fn try_recv(&mut self) -> Option<T> {
        match self.buffer.try_recv() {
            Ok(Some(t)) => {
                self.received = true;
                self.buffer.close();
                Some(t)
            }
            Ok(None) => None,
            Err(e) => panic!("the sender was dropped without sending ({e})"),
        }
    }
}
