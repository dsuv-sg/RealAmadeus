//! Producer side of `/chat/stream`: events go through an [`EventSink`] into a bounded channel;
//! the consumer side is an [`EventStream`] that aborts the producer task when dropped, which
//! cancels the upstream request (client disconnect / desktop cancel, V1 quirk Q21).

use std::pin::Pin;
use std::task::{Context, Poll};

use amadeus_core::protocol::StreamEvent;
use futures::stream::{BoxStream, Stream};
use serde_json::Value;
use tokio::sync::mpsc;

use super::payload::Thrown;
use crate::Backend;

/// Channel capacity between the producer task and the HTTP/IPC writer.
const CHANNEL_CAPACITY: usize = 64;

/// The consumer of the stream went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Closed;

/// Why a streaming producer stopped early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamErr {
    /// Consumer gone: stop silently.
    Closed,
    /// A JS exception, handled by the enclosing V1 `catch`.
    Thrown(String),
}

impl From<Closed> for StreamErr {
    fn from(_: Closed) -> Self {
        StreamErr::Closed
    }
}

impl From<Thrown> for StreamErr {
    fn from(t: Thrown) -> Self {
        StreamErr::Thrown(t.0)
    }
}

/// The `/chat/stream` enqueue interceptor (§7.2 step a): logs non-empty `complete` events
/// before they are written. Installed on the web server only.
pub(crate) struct Interceptor {
    pub(crate) backend: Backend,
    pub(crate) ip: String,
    pub(crate) provider: i64,
    pub(crate) question: Value,
    pub(crate) count: i64,
}

/// Where producers write their events.
pub(crate) struct EventSink {
    tx: mpsc::Sender<StreamEvent>,
    interceptor: Option<Interceptor>,
}

impl EventSink {
    /// Send one event (one SSE frame); logs it first when it is a non-empty completion.
    pub(crate) async fn emit(&mut self, event: StreamEvent) -> Result<(), Closed> {
        if let (Some(i), StreamEvent::Complete { full_response }) = (&self.interceptor, &event) {
            if !full_response.is_empty() {
                let reply = Value::String(full_response.clone());
                super::log_answer(&i.backend, &i.ip, i.provider, &i.question, &reply, i.count);
            }
        }
        self.tx.send(event).await.map_err(|_| Closed)
    }
}

/// Consumer half: yields the events and aborts the producer when dropped.
pub(crate) struct EventStream {
    rx: mpsc::Receiver<StreamEvent>,
    task: tokio::task::AbortHandle,
}

impl Stream for EventStream {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<StreamEvent>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for EventStream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Spawn `producer` on the tokio runtime and return the `'static` event stream it feeds.
pub(crate) fn spawn_stream<F, Fut>(
    interceptor: Option<Interceptor>,
    producer: F,
) -> BoxStream<'static, StreamEvent>
where
    F: FnOnce(EventSink) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let task = tokio::spawn(producer(EventSink { tx, interceptor }));
    Box::pin(EventStream {
        rx,
        task: task.abort_handle(),
    })
}

/// A finished stream made of fixed events (e.g. the rate-limited FreeAPI reply; not logged).
pub(crate) fn fixed_stream(events: Vec<StreamEvent>) -> BoxStream<'static, StreamEvent> {
    Box::pin(futures::stream::iter(events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn events_flow_and_drop_aborts_producer() {
        let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
        let mut stream = spawn_stream(None, |mut sink| async move {
            let _guard = DropSignal(Some(done_tx));
            let _ = sink.emit(StreamEvent::token("a")).await;
            // Block forever, like an upstream that never answers.
            std::future::pending::<()>().await;
        });
        assert_eq!(stream.next().await, Some(StreamEvent::token("a")));
        drop(stream);
        let dropped = tokio::time::timeout(std::time::Duration::from_secs(5), done_rx).await;
        assert!(dropped.is_ok(), "producer was not aborted");
    }

    #[tokio::test]
    async fn emit_fails_when_consumer_is_gone() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let mut sink = EventSink {
            tx,
            interceptor: None,
        };
        assert_eq!(sink.emit(StreamEvent::token("x")).await, Err(Closed));
    }

    struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }
}
