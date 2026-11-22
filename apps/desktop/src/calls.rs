//! Backend calls of the current page: a page (re)load aborts the calls the previous page left
//! running, and Tauri's re-send of an abandoned invoke never runs a second time.
//!
//! Tauri sends every invoke as a `fetch` to its IPC custom protocol (`ipc://localhost`,
//! `http://ipc.localhost` on Windows). When that `fetch` fails, Tauri's `ipc-protocol.js` sends the
//! same message again through `window.ipc.postMessage`. A page that reloads or navigates aborts its
//! pending `fetch`es, so without this module every backend call still in flight at that moment
//! would run twice with the same arguments (a second provider request for `chat`, a second stream
//! for `chat_stream`, …), and the copy would run for nobody.
//!
//! * [`CallRegistry::admit`] recognises the re-send: it arrives through `postMessage`
//!   ([`Delivery::PostMessage`]) with the same command and arguments ([`CallKey`]) as a
//!   custom-protocol call that is still running or ended less than [`RESEND_WINDOW`] ago. It is not
//!   run again: it waits for that call's outcome and returns it. Each call absorbs at most one
//!   re-send (Tauri re-sends a message once). A `postMessage` call without such a twin runs
//!   normally, so a webview whose custom protocol never works keeps working.
//! * [`CallRegistry::abort_all`] runs when a page load commits (`PageLoadEvent::Started`, before
//!   the new page's scripts run): the calls of the page that went away are aborted, as the web
//!   server drops a request whose browser went away.

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    hash::{DefaultHasher, Hasher},
    io,
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError},
    time::{Duration, Instant},
};

use amadeus_core::protocol::ApiReply;
use futures::future::{BoxFuture, FutureExt, Shared};
use serde::Serialize;
use tauri::{
    http::HeaderMap,
    ipc::{CommandArg, CommandItem, InvokeError},
    Runtime,
};
use tokio::task::{AbortHandle, JoinError, JoinHandle};

/// Request header that only Tauri's custom-protocol IPC sends. An invoke delivered through
/// `postMessage` carries only the headers its caller passed to `invoke(…, { headers })`.
const INVOKE_KEY_HEADER: &str = "Tauri-Invoke-Key";

/// How long a finished custom-protocol call can still absorb its re-send. The re-send happens as
/// soon as the page starts unloading; when the page load commits first, it can arrive after the
/// call was aborted.
pub const RESEND_WINDOW: Duration = Duration::from_secs(30);

/// Delay after [`RESEND_WINDOW`] before a finished call is forgotten by the timer (see
/// [`schedule_prune`]).
const PRUNE_SLACK: Duration = Duration::from_secs(1);

/// Finished calls kept for re-send matching beyond this number are forgotten, oldest first.
/// Running calls are always kept.
const MAX_RESENDABLE: usize = 64;

/// How an invoke reached the shell (a command argument that the frontend does not pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Tauri's custom-protocol IPC: the normal path.
    CustomProtocol,
    /// `window.ipc.postMessage`: Tauri's fallback, which also re-sends a call whose custom-protocol
    /// `fetch` failed.
    PostMessage,
}

impl Delivery {
    fn of(headers: &HeaderMap) -> Self {
        if headers.contains_key(INVOKE_KEY_HEADER) {
            Self::CustomProtocol
        } else {
            Self::PostMessage
        }
    }
}

impl<'de, R: Runtime> CommandArg<'de, R> for Delivery {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        Ok(Self::of(command.message.headers()))
    }
}

/// What a backend command resolves to: `chat_stream` resolves to `None` once its stream ran, the
/// other backend commands always to a reply.
pub type Outcome = Option<ApiReply>;

/// The outcome of one call, awaited by its own command and by a re-send of it.
pub type SharedOutcome = Shared<BoxFuture<'static, Outcome>>;

/// Hash of a command name and its arguments. A re-send carries exactly the arguments of the
/// original invoke (including the id of a `Channel`), so equal keys identify it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallKey(u64);

impl CallKey {
    /// Starts a key for `command`; add the arguments with [`CallKeyBuilder`].
    pub fn of(command: &str) -> CallKeyBuilder {
        let mut hasher = DefaultHasher::new();
        hasher.write(command.as_bytes());
        hasher.write_u8(0xff);
        CallKeyBuilder(hasher)
    }
}

/// Adds the arguments of a call to its [`CallKey`].
pub struct CallKeyBuilder(DefaultHasher);

impl CallKeyBuilder {
    /// Raw bytes (e.g. a JSON body that is serialized anyway).
    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        self.0.write_usize(bytes.len());
        self.0.write(bytes);
        self
    }

    /// The JSON serialization of `value`, streamed into the hash.
    pub fn json(mut self, value: &impl Serialize) -> Self {
        // Cannot fail for the JSON-shaped command arguments (string keys only).
        let _ = serde_json::to_writer(HashWriter(&mut self.0), value);
        self.0.write_u8(0xff);
        self
    }

    pub fn finish(self) -> CallKey {
        CallKey(self.0.finish())
    }
}

struct HashWriter<'a>(&'a mut DefaultHasher);

impl io::Write for HashWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Spawns `fut` on Tauri's tokio runtime and returns the tokio handle (for `abort_handle`).
pub fn spawn_task<F>(fut: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let tauri::async_runtime::JoinHandle::Tokio(task) = tauri::async_runtime::spawn(fut);
    task
}

/// Prunes the registry once a call that just finished can no longer be re-sent. Without it, the
/// last finished calls and their replies (TTS audio included) would stay in memory until the next
/// call happens to prune.
fn schedule_prune(registry: &Arc<CallRegistry>) {
    let registry = Arc::downgrade(registry);
    spawn_task(async move {
        tokio::time::sleep(RESEND_WINDOW + PRUNE_SLACK).await;
        if let Some(registry) = registry.upgrade() {
            registry.lock().prune(Instant::now());
        }
    });
}

/// The backend calls of the current page (managed in [`crate::state::DesktopState`]).
#[derive(Default)]
pub struct CallRegistry {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    /// Tasks of running calls, aborted by [`CallRegistry::abort_all`].
    running: HashMap<u64, AbortHandle>,
    /// Custom-protocol calls whose re-send has not arrived, oldest first.
    resendable: VecDeque<Resendable>,
}

impl Inner {
    /// Forgets calls that can no longer be re-sent.
    fn prune(&mut self, now: Instant) {
        self.resendable.retain(|call| !call.expired(now));
        while self.resendable.len() > MAX_RESENDABLE {
            match self.resendable.iter().position(Resendable::is_finished) {
                Some(index) => {
                    self.resendable.remove(index);
                }
                // Only running calls are left; their number is bounded by the page's requests.
                None => break,
            }
        }
    }
}

struct Resendable {
    key: CallKey,
    outcome: SharedOutcome,
    finished: Arc<OnceLock<Instant>>,
}

impl Resendable {
    fn is_finished(&self) -> bool {
        self.finished.get().is_some()
    }

    fn expired(&self, now: Instant) -> bool {
        self.finished
            .get()
            .is_some_and(|at| now.saturating_duration_since(*at) > RESEND_WINDOW)
    }
}

/// Result of [`CallRegistry::admit`].
pub enum Admission {
    /// A new call: start it with [`NewCall::spawn`] (or drop it if the call cannot start).
    New(NewCall),
    /// Tauri's re-send of a call that is running or just ended. Nothing runs; the command returns
    /// the original's outcome.
    Resent(SharedOutcome),
}

/// Permission to start a call (see [`CallRegistry::admit`]).
#[must_use = "start the call with `spawn`"]
pub struct NewCall {
    registry: Arc<CallRegistry>,
    key: CallKey,
    delivery: Delivery,
}

/// A started call.
pub struct RunningCall {
    /// Aborts the call's task; its outcome then comes from `on_join_error`.
    pub abort: AbortHandle,
    pub outcome: SharedOutcome,
}

impl CallRegistry {
    /// Decides whether an invoke starts a call or is Tauri's re-send of one.
    pub fn admit(self: &Arc<Self>, delivery: Delivery, key: CallKey) -> Admission {
        if delivery == Delivery::PostMessage {
            let mut inner = self.lock();
            inner.prune(Instant::now());
            if let Some(index) = inner.resendable.iter().position(|call| call.key == key) {
                // Removed: a call absorbs one re-send.
                if let Some(original) = inner.resendable.remove(index) {
                    return Admission::Resent(original.outcome);
                }
            }
        }
        Admission::New(NewCall {
            registry: Arc::clone(self),
            key,
            delivery,
        })
    }

    /// Aborts every running call (the page that made them went away). Their re-sends are still
    /// recognised for [`RESEND_WINDOW`]. Returns how many calls were running.
    pub fn abort_all(&self) -> usize {
        let running: Vec<AbortHandle> = self.lock().running.drain().map(|(_, a)| a).collect();
        for task in &running {
            task.abort();
        }
        running.len()
    }

    #[cfg(test)]
    fn running_count(&self) -> usize {
        self.lock().running.len()
    }

    #[cfg(test)]
    fn resendable_count(&self) -> usize {
        self.lock().resendable.len()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // The maps stay consistent even if a holder panicked, so poisoning is ignored.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl NewCall {
    /// Runs `work` on its own task. A panic or an abort of that task becomes the outcome
    /// `on_join_error` returns.
    pub fn spawn<W, E>(self, work: W, on_join_error: E) -> RunningCall
    where
        W: Future<Output = Outcome> + Send + 'static,
        E: FnOnce(JoinError) -> Outcome + Send + 'static,
    {
        let registry = self.registry;
        // Only a custom-protocol invoke can be re-sent; a `postMessage` one has no fallback.
        let resendable = self.delivery == Delivery::CustomProtocol;
        // Spawned and registered under the lock, so a concurrent `abort_all` sees the call.
        let (task, abort, id) = {
            let mut inner = registry.lock();
            let task = spawn_task(work);
            let abort = task.abort_handle();
            let id = inner.next_id;
            inner.next_id = inner.next_id.wrapping_add(1);
            inner.running.insert(id, abort.clone());
            (task, abort, id)
        };
        let finished = Arc::new(OnceLock::new());
        let outcome = {
            let registry = Arc::clone(&registry);
            let finished = Arc::clone(&finished);
            async move {
                let result = task.await;
                registry.lock().running.remove(&id);
                let _ = finished.set(Instant::now());
                if resendable {
                    schedule_prune(&registry);
                }
                match result {
                    Ok(outcome) => outcome,
                    Err(err) => on_join_error(err),
                }
            }
            .boxed()
            .shared()
        };
        if resendable {
            let mut inner = registry.lock();
            inner.prune(Instant::now());
            inner.resendable.push_back(Resendable {
                key: self.key,
                outcome: outcome.clone(),
                finished,
            });
        }
        RunningCall { abort, outcome }
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::*;

    fn key(n: u32) -> CallKey {
        CallKey::of("chat").json(&n).finish()
    }

    fn reply(status: u16) -> Outcome {
        Some(ApiReply::error(status, "x"))
    }

    fn start(
        registry: &Arc<CallRegistry>,
        delivery: Delivery,
        key: CallKey,
        outcome: Outcome,
    ) -> RunningCall {
        match registry.admit(delivery, key) {
            Admission::New(call) => call.spawn(async move { outcome }, |_| reply(500)),
            Admission::Resent(_) => panic!("unexpected re-send match"),
        }
    }

    fn start_pending(
        registry: &Arc<CallRegistry>,
        delivery: Delivery,
        key: CallKey,
    ) -> RunningCall {
        match registry.admit(delivery, key) {
            Admission::New(call) => call.spawn(pending::<Outcome>(), |err| {
                assert!(err.is_cancelled());
                reply(499)
            }),
            Admission::Resent(_) => panic!("unexpected re-send match"),
        }
    }

    #[test]
    fn keys_cover_the_command_and_every_argument() {
        let a = CallKey::of("chat").bytes(b"{}").finish();
        assert_eq!(a, CallKey::of("chat").bytes(b"{}").finish());
        assert_ne!(a, CallKey::of("rag_embed").bytes(b"{}").finish());
        assert_ne!(a, CallKey::of("chat").bytes(b"{ }").finish());
        let stream = |id: u32, channel: u32| {
            CallKey::of("chat_stream")
                .json(&id)
                .json(&channel)
                .bytes(b"{}")
                .finish()
        };
        assert_eq!(stream(1, 7), stream(1, 7));
        assert_ne!(stream(1, 7), stream(1, 8));
        assert_ne!(stream(1, 7), stream(2, 7));
        // Byte boundaries are part of the key.
        assert_ne!(
            CallKey::of("x").bytes(b"ab").bytes(b"c").finish(),
            CallKey::of("x").bytes(b"a").bytes(b"bc").finish()
        );
    }

    #[tokio::test]
    async fn a_resend_of_a_running_call_replays_its_outcome() {
        let registry = Arc::new(CallRegistry::default());
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let original = match registry.admit(Delivery::CustomProtocol, key(1)) {
            Admission::New(call) => call.spawn(
                async move {
                    let _ = rx.await;
                    reply(201)
                },
                |_| reply(500),
            ),
            Admission::Resent(_) => unreachable!(),
        };
        assert_eq!(registry.running_count(), 1);

        let Admission::Resent(replayed) = registry.admit(Delivery::PostMessage, key(1)) else {
            panic!("the re-send was not recognised");
        };
        assert_eq!(registry.running_count(), 1, "the re-send started nothing");
        tx.send(()).unwrap();
        assert_eq!(replayed.await, reply(201));
        assert_eq!(original.outcome.await, reply(201));
        assert_eq!(registry.running_count(), 0);

        // A call absorbs one re-send: the next identical postMessage call is a new call.
        assert!(matches!(
            registry.admit(Delivery::PostMessage, key(1)),
            Admission::New(_)
        ));
    }

    #[tokio::test]
    async fn a_resend_after_the_call_ended_gets_its_outcome() {
        let registry = Arc::new(CallRegistry::default());
        let original = start(&registry, Delivery::CustomProtocol, key(2), reply(200));
        assert_eq!(original.outcome.await, reply(200));
        let Admission::Resent(replayed) = registry.admit(Delivery::PostMessage, key(2)) else {
            panic!("a re-send right after the call ended was not recognised");
        };
        assert_eq!(replayed.await, reply(200));
    }

    #[tokio::test]
    async fn only_custom_protocol_calls_can_be_resent() {
        let registry = Arc::new(CallRegistry::default());
        // Different arguments: no match.
        let other = start(&registry, Delivery::CustomProtocol, key(3), reply(200));
        assert!(matches!(
            registry.admit(Delivery::PostMessage, key(4)),
            Admission::New(_)
        ));
        // A custom-protocol call is never treated as a re-send, even with equal arguments.
        assert!(matches!(
            registry.admit(Delivery::CustomProtocol, key(3)),
            Admission::New(_)
        ));
        // A postMessage call (custom protocol unavailable) cannot be re-sent.
        let fallback = start(&registry, Delivery::PostMessage, key(5), reply(200));
        assert!(matches!(
            registry.admit(Delivery::PostMessage, key(5)),
            Admission::New(_)
        ));
        assert_eq!(registry.resendable_count(), 1);
        other.outcome.await;
        fallback.outcome.await;
    }

    #[tokio::test]
    async fn abort_all_ends_running_calls_through_on_join_error() {
        let registry = Arc::new(CallRegistry::default());
        let a = start_pending(&registry, Delivery::CustomProtocol, key(6));
        let b = start_pending(&registry, Delivery::PostMessage, key(7));
        assert_eq!(registry.abort_all(), 2);
        assert_eq!(a.outcome.await, reply(499));
        assert_eq!(b.outcome.await, reply(499));
        assert_eq!(registry.running_count(), 0);
        assert_eq!(registry.abort_all(), 0);

        // The aborted call's re-send (page load committed before it arrived) gets the outcome.
        let Admission::Resent(replayed) = registry.admit(Delivery::PostMessage, key(6)) else {
            panic!("the re-send of an aborted call was not recognised");
        };
        assert_eq!(replayed.await, reply(499));
    }

    #[tokio::test]
    async fn finished_calls_are_forgotten_beyond_the_limit_but_running_ones_are_kept() {
        let registry = Arc::new(CallRegistry::default());
        let running = start_pending(&registry, Delivery::CustomProtocol, key(1000));
        for n in 0..(MAX_RESENDABLE as u32 + 10) {
            start(&registry, Delivery::CustomProtocol, key(n), reply(200))
                .outcome
                .await;
        }
        assert!(registry.resendable_count() <= MAX_RESENDABLE + 1);
        assert!(
            matches!(
                registry.admit(Delivery::PostMessage, key(1000)),
                Admission::Resent(_)
            ),
            "the running call must stay re-sendable"
        );
        assert!(
            matches!(
                registry.admit(Delivery::PostMessage, key(0)),
                Admission::New(_)
            ),
            "the oldest finished call is forgotten"
        );
        running.abort.abort();
        running.outcome.await;
    }

    #[test]
    fn expiry() {
        let finished = Arc::new(OnceLock::new());
        let call = Resendable {
            key: key(1),
            outcome: async { None }.boxed().shared(),
            finished: Arc::clone(&finished),
        };
        let now = Instant::now();
        assert!(
            !call.expired(now + RESEND_WINDOW * 10),
            "running calls never expire"
        );
        finished.set(now).unwrap();
        assert!(!call.expired(now + RESEND_WINDOW));
        assert!(call.expired(now + RESEND_WINDOW + Duration::from_millis(1)));
    }
}
