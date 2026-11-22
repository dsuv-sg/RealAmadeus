//! Managed state: the shared backend, the backend calls of the current page and the registry of
//! in-flight chat streams.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use amadeus_backend::Backend;
use tokio::task::AbortHandle;

use crate::calls::CallRegistry;

/// How many recently settled / early-cancelled stream ids the registry remembers.
const RECENT_IDS: usize = 64;

/// Everything the IPC commands share. Managed once in `setup` (see `lib.rs`).
pub struct DesktopState {
    /// `Err` holds the reason the backend could not be created; commands then reply with 500.
    backend: Result<Backend, String>,
    remote_api_base: String,
    calls: Arc<CallRegistry>,
    streams: Arc<StreamRegistry>,
}

impl DesktopState {
    pub fn new(backend: Result<Backend, String>, remote_api_base: String) -> Self {
        Self {
            backend,
            remote_api_base,
            calls: Arc::default(),
            streams: Arc::default(),
        }
    }

    /// Cheap clone of the backend handle (everything is behind an `Arc`).
    pub fn backend(&self) -> Result<Backend, String> {
        self.backend.clone()
    }

    pub fn remote_api_base(&self) -> &str {
        &self.remote_api_base
    }

    /// Every backend call of the current page (aborted on reload, re-sends recognised).
    pub fn calls(&self) -> &Arc<CallRegistry> {
        &self.calls
    }

    /// `chat_stream` tasks by `streamId` (for `chat_stream_cancel`).
    pub fn streams(&self) -> &Arc<StreamRegistry> {
        &self.streams
    }
}

/// In-flight `chat_stream` tasks keyed by the frontend's `streamId`, so `chat_stream_cancel`
/// can abort them. Every entry is removed when its stream ends or is cancelled.
///
/// The two commands are separate invokes whose tasks Tauri spawns independently, so a cancel
/// can overtake the `chat_stream` it targets. A cancel for an id that is neither running nor
/// recently settled is therefore remembered, and that stream is aborted as soon as it registers.
#[derive(Default)]
pub struct StreamRegistry {
    inner: Mutex<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
    next_seq: u64,
    /// Incremented by [`StreamRegistry::cancel_all`] (page reload): streams of an older page
    /// never touch the ids of the new one.
    page: u64,
    active: HashMap<u32, ActiveStream>,
    /// Ids whose stream ended or was cancelled: a late cancel for them is a no-op, not an
    /// early cancel.
    settled: RecentIds,
    /// Ids cancelled before their stream registered.
    cancelled_early: RecentIds,
}

struct ActiveStream {
    seq: u64,
    abort: AbortHandle,
}

/// Small FIFO set of stream ids (oldest evicted beyond [`RECENT_IDS`]).
#[derive(Default)]
struct RecentIds(VecDeque<u32>);

impl RecentIds {
    fn contains(&self, id: u32) -> bool {
        self.0.contains(&id)
    }

    fn insert(&mut self, id: u32) {
        if !self.contains(id) {
            if self.0.len() >= RECENT_IDS {
                self.0.pop_front();
            }
            self.0.push_back(id);
        }
    }

    /// Removes `id`; `true` if it was present.
    fn take(&mut self, id: u32) -> bool {
        match self.0.iter().position(|&x| x == id) {
            Some(index) => {
                self.0.remove(index);
                true
            }
            None => false,
        }
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

/// Identifies one registration, so a finished stream never unregisters a newer stream that
/// reused its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamTicket {
    stream_id: u32,
    seq: u64,
    page: u64,
}

impl StreamRegistry {
    /// Registers a running stream task. If the stream was already cancelled (the cancel
    /// overtook it), the task is aborted right away. A still-running stream with the same id is
    /// a stale leftover (ids must be unique among in-flight streams), so it is cancelled.
    pub fn register(&self, stream_id: u32, abort: AbortHandle) -> StreamTicket {
        let mut inner = self.lock();
        let ticket = StreamTicket {
            stream_id,
            seq: inner.next_seq,
            page: inner.page,
        };
        inner.next_seq = inner.next_seq.wrapping_add(1);
        // The id is in use again: a later cancel targets this stream.
        inner.settled.take(stream_id);
        if inner.cancelled_early.take(stream_id) {
            inner.settled.insert(stream_id);
            drop(inner);
            log::info!("chat stream {stream_id} was cancelled before it started");
            abort.abort();
            return ticket;
        }
        let previous = inner.active.insert(
            stream_id,
            ActiveStream {
                seq: ticket.seq,
                abort,
            },
        );
        drop(inner);
        // Abort outside the lock.
        if let Some(previous) = previous {
            log::warn!("chat stream {stream_id} started while an older stream with that id was running; cancelling the older one");
            previous.abort.abort();
        }
        ticket
    }

    /// Aborts and unregisters the stream. Returns `false` when no such stream is running; if it
    /// has not started yet (and did not just settle), it is aborted when it registers.
    pub fn cancel(&self, stream_id: u32) -> bool {
        let removed = {
            let mut inner = self.lock();
            let removed = inner.active.remove(&stream_id);
            if removed.is_some() {
                inner.settled.insert(stream_id);
            } else if !inner.settled.contains(stream_id) {
                inner.cancelled_early.insert(stream_id);
            }
            removed
        };
        match removed {
            Some(stream) => {
                stream.abort.abort();
                true
            }
            None => false,
        }
    }

    /// Unregisters a stream that ended, unless its id has been taken over by a newer stream.
    pub fn finish(&self, ticket: StreamTicket) {
        let mut inner = self.lock();
        if ticket.page != inner.page {
            // Started before the last page reload, which already forgot it. Marking its id as
            // settled now would turn an early cancel of the new page's stream with that id into
            // a no-op.
            return;
        }
        match inner.active.get(&ticket.stream_id) {
            Some(stream) if stream.seq == ticket.seq => {
                inner.active.remove(&ticket.stream_id);
                inner.settled.insert(ticket.stream_id);
            }
            // A newer stream reused the id; it owns the entry now.
            Some(_) => {}
            // Already removed by `cancel` (or aborted on registration).
            None => inner.settled.insert(ticket.stream_id),
        }
    }

    /// Aborts every in-flight stream and forgets all ids (the page that owned their channels
    /// was reloaded; the new page may start counting stream ids again).
    pub fn cancel_all(&self) -> usize {
        let drained: Vec<ActiveStream> = {
            let mut inner = self.lock();
            inner.page = inner.page.wrapping_add(1);
            inner.settled.clear();
            inner.cancelled_early.clear();
            inner.active.drain().map(|(_, s)| s).collect()
        };
        for stream in &drained {
            stream.abort.abort();
        }
        drained.len()
    }

    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.lock().active.len()
    }

    fn lock(&self) -> MutexGuard<'_, RegistryInner> {
        // The map stays consistent even if a holder panicked, so poisoning is ignored.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Owned by a running `chat_stream` command: on drop (stream finished, cancelled, panicked,
/// or the command future itself dropped) the task is aborted and the entry removed.
pub struct StreamGuard {
    registry: Arc<StreamRegistry>,
    ticket: StreamTicket,
    abort: AbortHandle,
}

impl StreamGuard {
    pub fn new(registry: Arc<StreamRegistry>, stream_id: u32, abort: AbortHandle) -> Self {
        let ticket = registry.register(stream_id, abort.clone());
        Self {
            registry,
            ticket,
            abort,
        }
    }
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        self.abort.abort();
        self.registry.finish(self.ticket);
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::*;

    fn pending_task() -> tokio::task::JoinHandle<()> {
        tokio::spawn(pending::<()>())
    }

    #[tokio::test]
    async fn cancel_aborts_and_unregisters() {
        let registry = Arc::new(StreamRegistry::default());
        let task = pending_task();
        let guard = StreamGuard::new(registry.clone(), 7, task.abort_handle());
        assert_eq!(registry.active_count(), 1);

        assert!(registry.cancel(7));
        assert_eq!(registry.active_count(), 0);
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!registry.cancel(7), "second cancel finds nothing");
        drop(guard);
        assert_eq!(registry.active_count(), 0);
    }

    #[tokio::test]
    async fn guard_drop_unregisters_only_its_own_entry() {
        let registry = Arc::new(StreamRegistry::default());
        let old_task = pending_task();
        let old_guard = StreamGuard::new(registry.clone(), 1, old_task.abort_handle());

        // Same id again: the older stream is cancelled, the newer one stays registered.
        let new_task = pending_task();
        let new_guard = StreamGuard::new(registry.clone(), 1, new_task.abort_handle());
        assert!(old_task.await.unwrap_err().is_cancelled());
        drop(old_guard);
        assert_eq!(
            registry.active_count(),
            1,
            "the older guard must not remove the newer entry"
        );

        drop(new_guard);
        assert_eq!(registry.active_count(), 0);
        assert!(
            new_task.await.unwrap_err().is_cancelled(),
            "dropping the guard aborts the task"
        );
    }

    #[tokio::test]
    async fn cancel_all_aborts_everything() {
        let registry = Arc::new(StreamRegistry::default());
        let tasks: Vec<_> = (0..3).map(|_| pending_task()).collect();
        let guards: Vec<_> = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| StreamGuard::new(registry.clone(), i as u32, t.abort_handle()))
            .collect();
        assert_eq!(registry.cancel_all(), 3);
        assert_eq!(registry.active_count(), 0);
        for task in tasks {
            assert!(task.await.unwrap_err().is_cancelled());
        }
        drop(guards);
    }

    #[tokio::test]
    async fn cancel_that_overtakes_its_stream_aborts_it_on_registration() {
        let registry = Arc::new(StreamRegistry::default());
        assert!(!registry.cancel(3), "nothing is running yet");

        let task = pending_task();
        let guard = StreamGuard::new(registry.clone(), 3, task.abort_handle());
        assert_eq!(
            registry.active_count(),
            0,
            "an early-cancelled stream is never registered"
        );
        assert!(task.await.unwrap_err().is_cancelled());
        drop(guard);

        // The early cancel is consumed: the next stream with that id runs normally.
        let next = pending_task();
        let next_guard = StreamGuard::new(registry.clone(), 3, next.abort_handle());
        assert_eq!(registry.active_count(), 1);
        drop(next_guard);
    }

    #[tokio::test]
    async fn late_cancel_of_a_finished_stream_does_not_poison_its_id() {
        let registry = Arc::new(StreamRegistry::default());
        let first = tokio::spawn(async {});
        let guard = StreamGuard::new(registry.clone(), 5, first.abort_handle());
        drop(guard); // the stream ended
        assert!(!registry.cancel(5), "late cancel finds nothing");

        // An id reused after completion must not be killed by that late cancel.
        let second = pending_task();
        let second_guard = StreamGuard::new(registry.clone(), 5, second.abort_handle());
        assert_eq!(registry.active_count(), 1);
        drop(second_guard);
        assert!(
            second.await.unwrap_err().is_cancelled(),
            "dropping the guard aborts the task"
        );
    }

    #[tokio::test]
    async fn page_reload_forgets_early_cancels() {
        let registry = Arc::new(StreamRegistry::default());
        assert!(!registry.cancel(1));
        registry.cancel_all();

        let task = pending_task();
        let guard = StreamGuard::new(registry.clone(), 1, task.abort_handle());
        assert_eq!(
            registry.active_count(),
            1,
            "the new page's stream 1 is not the old page's"
        );
        drop(guard);
    }

    #[tokio::test]
    async fn a_stream_of_the_previous_page_ending_late_does_not_settle_the_new_pages_id() {
        let registry = Arc::new(StreamRegistry::default());
        let old_task = pending_task();
        let old_guard = StreamGuard::new(registry.clone(), 1, old_task.abort_handle());
        registry.cancel_all(); // page reload
        drop(old_guard); // the aborted command of the old page returns afterwards

        // The new page cancels its stream 1 before the `chat_stream` invoke registered it.
        assert!(!registry.cancel(1));
        let task = pending_task();
        let guard = StreamGuard::new(registry.clone(), 1, task.abort_handle());
        assert_eq!(
            registry.active_count(),
            0,
            "the early cancel of the new page's stream must still apply"
        );
        assert!(task.await.unwrap_err().is_cancelled());
        drop(guard);
    }

    #[test]
    fn recent_ids_are_bounded() {
        let mut ids = RecentIds::default();
        for id in 0..(RECENT_IDS as u32 + 10) {
            ids.insert(id);
        }
        assert_eq!(ids.0.len(), RECENT_IDS);
        assert!(!ids.contains(0), "the oldest ids are evicted");
        assert!(ids.contains(RECENT_IDS as u32 + 9));
        assert!(ids.take(RECENT_IDS as u32 + 9));
        assert!(!ids.take(RECENT_IDS as u32 + 9));
    }
}
