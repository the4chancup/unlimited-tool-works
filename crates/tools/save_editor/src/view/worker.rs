//! A worker-thread computation's cycle, shared by the AATF checks
//! (`aatf_check`), the comparator (`comparator`) and the pitch badges'
//! medals (`tactics`): a key naming what a run is
//! for, a reply channel polled before a new run is decided on, and the last
//! result held while a run is in flight or due — so the frame that starts a
//! run still marks the previous result as waiting, and a worker that ends
//! without a reply leaves the next settled frame to start a fresh one.

use crossbeam_channel::Receiver;

/// A run in flight: the key it runs for and its reply channel.
pub(super) struct Pending<K, R> {
    /// The key the worker runs for.
    pub(super) key: K,
    /// The worker's reply: its key and its result.
    pub(super) replies: Receiver<(K, R)>,
}

/// A worker computation's state: the last run's result with the key it ran
/// for, and the run in flight, if one is.
pub(super) struct WorkerResult<K, R> {
    /// The last run's key and result.
    pub(super) result: Option<(K, R)>,
    /// The run in flight, if one is.
    pub(super) pending: Option<Pending<K, R>>,
}

impl<K: PartialEq, R> WorkerResult<K, R> {
    /// Nothing run and nothing in flight.
    pub(super) fn new() -> WorkerResult<K, R> {
        WorkerResult {
            result: None,
            pending: None,
        }
    }

    /// Another save opened: the result and the run in flight are of the old
    /// save.
    pub(super) fn reset(&mut self) {
        self.result = None;
        self.pending = None;
    }

    /// One frame of the cycle for `key`: the reply polled first, so the frame
    /// that starts a run still marks the previous result as waiting; then a
    /// run spawned when `ready` (the caller's inputs are in place), the key
    /// moved on since the last result, and the edit has settled. Returns
    /// whether a run is in flight or due at the next settled frame — the
    /// callers' waiting mark.
    pub(super) fn update(
        &mut self,
        key: &K,
        ready: bool,
        settled: bool,
        spawn: impl FnOnce(&K) -> Pending<K, R>,
    ) -> bool {
        let reply = self
            .pending
            .as_ref()
            .map(|pending| pending.replies.try_recv());
        match reply {
            Some(Ok((key, result))) => {
                self.result = Some((key, result));
                self.pending = None;
            }
            // A worker that died without replying leaves no result to wait
            // for: drop the pending so the spawn below starts a fresh run.
            Some(Err(crossbeam_channel::TryRecvError::Disconnected)) => {
                log::warn!("a view's worker ended without a reply");
                self.pending = None;
            }
            _ => {}
        }
        let due = ready && self.result.as_ref().is_none_or(|(held, _)| *held != *key);
        let pending_for_key = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.key == *key);
        if due && settled && !pending_for_key {
            self.pending = Some(spawn(key));
        }
        self.pending.is_some() || due
    }
}
