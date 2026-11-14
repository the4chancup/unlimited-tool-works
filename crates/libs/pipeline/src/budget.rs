//! The run's memory budget: a task acquires its source bytes before loading,
//! charges what it allocates while running without waiting, and the writer
//! releases the batch's permits by dropping them. An oversized request waits
//! for an empty budget rather than for capacity it can never get, and while
//! one waits, ordinary requests wait behind it — otherwise `in_flight` never
//! reaches zero.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// The share of a budget's cap that archives kept decompressed from their check to their
/// tasks must leave free: 1 GiB. A task's charges do not wait for room (`charge`), so kept
/// archives filling the budget would stall the very tasks that release them; the value is
/// above the largest texture decode real exports need, an 8192x8192 texture with its mips
/// and its source file (measured at 427 MiB), so only a larger texture takes the run past the cap
/// while kept archives are held (`spec/libs/pipeline.md` "What a solid `.7z` is charged").
pub const MIN_FREE_BUDGET: usize = 1 << 30;

/// A byte budget shared by a run's tasks: `acquire` blocks until the bytes
/// fit.
pub struct MemoryBudget {
    cap: usize,
    state: Mutex<BudgetState>,
    notify: Condvar,
}

struct BudgetState {
    in_flight: usize,
    /// The most `in_flight` has been since the budget was made.
    peak: usize,
    /// Oversized requests waiting for the pipeline to drain.
    oversized_waiting: usize,
    cancelled: bool,
}

/// Bytes held from a `MemoryBudget`, released when dropped.
pub struct Permit {
    budget: Arc<MemoryBudget>,
    size: usize,
}

/// The run was cancelled while (or before) waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the run was cancelled")]
pub struct Cancelled;

impl MemoryBudget {
    /// A budget of `cap` bytes; shared via `Arc` because a permit outlives its
    /// acquiring thread (it travels with the batch to the writer).
    pub fn new(cap: usize) -> Arc<MemoryBudget> {
        Arc::new(MemoryBudget {
            cap,
            state: Mutex::new(BudgetState {
                in_flight: 0,
                peak: 0,
                oversized_waiting: 0,
                cancelled: false,
            }),
            notify: Condvar::new(),
        })
    }

    /// The budget's cap, the bytes `acquire` fits requests into.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Blocks until `size` bytes fit (or, for a request over the cap, until
    /// the pipeline is empty), then charges them. `Err(Cancelled)` when the
    /// budget is cancelled before or during the wait; no bytes are taken.
    pub fn acquire(self: &Arc<Self>, size: usize) -> Result<Permit, Cancelled> {
        let mut state = self.lock();
        if size > self.cap {
            // An oversized request waits for the pipeline to drain, then
            // proceeds alone — waiting for ordinary capacity would deadlock.
            state.oversized_waiting += 1;
            while !state.cancelled && state.in_flight > 0 {
                state = self
                    .notify
                    .wait(state)
                    .expect("nothing panics while holding the budget lock");
            }
            state.oversized_waiting -= 1;
        } else {
            // `oversized_waiting` is what lets an oversized request ever run:
            // without it, ordinary arrivals keep `in_flight` above zero for
            // the whole run.
            while !state.cancelled
                && (state.oversized_waiting > 0
                    || state
                        .in_flight
                        .checked_add(size)
                        .is_none_or(|total| total > self.cap))
            {
                state = self
                    .notify
                    .wait(state)
                    .expect("nothing panics while holding the budget lock");
            }
        }
        if state.cancelled {
            return Err(Cancelled);
        }
        state.take(size);
        Ok(Permit {
            budget: Arc::clone(self),
            size,
        })
    }

    /// Takes `size` bytes at once, without waiting: for memory a running task
    /// allocates (a decoded image, a parsed model, its packed entries). A task
    /// that holds a permit and waits for more can wait for bytes only its own
    /// completion releases, and every worker can be in that state at once, so
    /// a charge never waits, whatever the cap, an oversized waiter or a
    /// cancellation. `in_flight` may then pass the cap by what the running
    /// tasks hold, and every later `acquire` waits for it to drain.
    pub fn charge(self: &Arc<Self>, size: usize) -> Permit {
        self.lock().take(size);
        Permit {
            budget: Arc::clone(self),
            size,
        }
    }

    /// Wakes every waiter; every later `acquire` returns `Cancelled`.
    pub fn cancel(&self) {
        self.lock().cancelled = true;
        self.notify.notify_all();
    }

    /// Whether `cancel` was called: a caller about to start work that acquires inside a
    /// callee (an archive read) checks it first.
    pub fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    /// The most bytes held at once since the budget was made, through
    /// `acquire` and `charge` alike: what the tests measure.
    pub fn peak(&self) -> usize {
        self.lock().peak
    }

    /// Nothing panics while holding the lock, so a poisoned mutex is a bug in
    /// this module.
    fn lock(&self) -> MutexGuard<'_, BudgetState> {
        self.state
            .lock()
            .expect("nothing panics while holding the budget lock")
    }

    #[cfg(test)]
    fn in_flight(&self) -> usize {
        self.lock().in_flight
    }

    #[cfg(test)]
    fn oversized_waiting(&self) -> usize {
        self.lock().oversized_waiting
    }
}

impl BudgetState {
    /// Adds `size` to `in_flight` and raises `peak`: the one place both
    /// `acquire` and `charge` take bytes, so the two cannot drift.
    fn take(&mut self, size: usize) {
        self.in_flight = self
            .in_flight
            .checked_add(size)
            .expect("bytes held at once fit in a usize: an overflow is a miscounted charge");
        self.peak = self.peak.max(self.in_flight);
    }
}

impl Permit {
    /// The bytes this permit holds.
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut state = self.budget.lock();
        assert!(
            state.in_flight >= self.size,
            "memory-budget permit released twice"
        );
        state.in_flight -= self.size;
        // An ordinary waiter and an oversized waiter wait for different
        // conditions on the one Condvar, so every release wakes them all.
        self.budget.notify.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
    use std::thread;
    use std::time::Duration;

    /// Long enough that a scheduling hiccup never outlives it; short enough
    /// that a lost wake-up fails the test rather than hanging it.
    const GUARD: Duration = Duration::from_secs(120);
    /// How long "still waiting" takes to observe.
    const BLOCKED: Duration = Duration::from_millis(100);

    /// A thread that acquires `size`, reports, holds the permit until
    /// `release` is dropped, then reports the release.
    struct Waiter {
        acquired: Receiver<Result<(), Cancelled>>,
        released: Receiver<()>,
        release: Sender<()>,
    }

    fn waiter(budget: &Arc<MemoryBudget>, size: usize) -> Waiter {
        let (acquired_tx, acquired) = channel();
        let (release, release_rx) = channel::<()>();
        let (released_tx, released) = channel();
        let budget = Arc::clone(budget);
        thread::spawn(move || {
            let permit = budget.acquire(size);
            acquired_tx
                .send(permit.as_ref().map(|_| ()).map_err(|e| *e))
                .unwrap();
            if let Ok(permit) = permit {
                // The sender going away releases the hold.
                while release_rx.recv().is_ok() {}
                drop(permit);
                released_tx.send(()).unwrap();
            }
        });
        Waiter {
            acquired,
            released,
            release,
        }
    }

    impl Waiter {
        /// The waiter acquired (or was cancelled) within `GUARD`.
        fn acquired(&self) -> Result<(), Cancelled> {
            self.acquired
                .recv_timeout(GUARD)
                .expect("a waiter must report within GUARD")
        }

        /// The waiter is still waiting after `BLOCKED`.
        fn blocked(&self) {
            assert!(
                matches!(
                    self.acquired.recv_timeout(BLOCKED),
                    Err(RecvTimeoutError::Timeout)
                ),
                "the waiter was not blocked"
            );
        }

        /// Lets the held permit go and confirms it released.
        fn release(self) {
            drop(self.release);
            self.released
                .recv_timeout(GUARD)
                .expect("a released waiter must report within GUARD");
        }
    }

    /// `budget.charge(size)` on another thread, which must return within `GUARD`: a charge
    /// that waited would hang the test instead of failing it.
    fn charged(budget: &Arc<MemoryBudget>, size: usize) -> Permit {
        let (charged_tx, charged) = channel();
        let budget = Arc::clone(budget);
        thread::spawn(move || charged_tx.send(budget.charge(size)).unwrap());
        charged
            .recv_timeout(GUARD)
            .expect("a charge must return at once, whatever the budget holds")
    }

    #[test]
    fn a_request_that_fits_acquires_immediately() {
        let budget = MemoryBudget::new(10);
        let permit = budget.acquire(5).unwrap();
        assert_eq!(permit.size(), 5);
        assert_eq!(budget.in_flight(), 5);
        drop(permit);
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn an_ordinary_request_waits_for_room_and_wakes_on_release() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(80).unwrap();
        let waiter = waiter(&budget, 50);
        waiter.blocked();
        drop(held);
        assert_eq!(waiter.acquired(), Ok(()));
        waiter.release();
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn an_oversized_request_waits_for_empty_and_runs_alone() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(80).unwrap();
        let oversized = waiter(&budget, 200);
        // The ordinary request must observe the oversized one registered as
        // waiting — a sleep here would only hope the spawn scheduled it.
        let deadline = std::time::Instant::now() + GUARD;
        while budget.oversized_waiting() == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "the oversized waiter never registered"
            );
            thread::yield_now();
        }
        // 80 + 20 fits the cap, but an oversized waiter goes first: the
        // ordinary request waits behind it or `in_flight` never drains.
        let ordinary = waiter(&budget, 20);
        oversized.blocked();
        ordinary.blocked();
        drop(held);
        assert_eq!(oversized.acquired(), Ok(()));
        assert_eq!(budget.in_flight(), 200);
        ordinary.blocked();
        oversized.release();
        assert_eq!(ordinary.acquired(), Ok(()));
        ordinary.release();
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn a_request_of_exactly_the_cap_is_ordinary() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(50).unwrap();
        // `size == cap` is an ordinary request: it waits for room like any
        // other, and it does not hold later waiters back.
        let big = waiter(&budget, 100);
        big.blocked();
        let fast = waiter(&budget, 10);
        assert_eq!(fast.acquired(), Ok(()));
        drop(held);
        // 10 + 100 still does not fit: `big` waits for `fast` to release.
        big.blocked();
        fast.release();
        assert_eq!(big.acquired(), Ok(()));
        big.release();
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn two_oversized_waiters_complete_one_at_a_time() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(10).unwrap();
        let (tx, rx) = channel();
        let mut releases = Vec::new();
        let mut joins = Vec::new();
        for size in [150usize, 160] {
            let budget = Arc::clone(&budget);
            let tx = tx.clone();
            let (release_tx, release_rx) = channel::<()>();
            releases.push(release_tx);
            joins.push(thread::spawn(move || {
                let permit = budget.acquire(size).unwrap();
                // Still holding: the runner sees the whole budget as its own.
                tx.send((size, budget.in_flight())).unwrap();
                while release_rx.recv().is_ok() {}
                drop(permit);
            }));
        }
        // Both wait for the pipeline to empty; an empty drain report before
        // then means one ran early.
        assert!(matches!(
            rx.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout)
        ));
        drop(held);
        let (size, in_flight) = rx.recv_timeout(GUARD).unwrap();
        assert_eq!(in_flight, size, "the oversized request did not run alone");
        assert!(matches!(
            rx.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout)
        ));
        drop(releases);
        let (size, in_flight) = rx.recv_timeout(GUARD).unwrap();
        assert_eq!(in_flight, size);
        for handle in joins {
            handle.join().unwrap();
        }
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn many_waiters_never_lose_a_wakeup() {
        let budget = MemoryBudget::new(64);
        let (done_tx, done_rx) = channel();
        let handles: Vec<_> = (0..8)
            .map(|thread_index| {
                let budget = Arc::clone(&budget);
                let done_tx = done_tx.clone();
                thread::spawn(move || {
                    for round in 0..200 {
                        // A mix of ordinary and oversized charges.
                        let size = if (thread_index + round) % 7 == 0 {
                            100
                        } else {
                            1 + (round % 30)
                        };
                        drop(budget.acquire(size).unwrap());
                    }
                    done_tx.send(()).unwrap();
                })
            })
            .collect();
        for _ in &handles {
            done_rx
                .recv_timeout(GUARD)
                .expect("every worker finishes within GUARD");
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn cancel_wakes_every_waiter_and_a_held_permit_still_releases() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(100).unwrap();
        let ordinary = waiter(&budget, 50);
        let oversized = waiter(&budget, 200);
        ordinary.blocked();
        oversized.blocked();
        budget.cancel();
        assert_eq!(ordinary.acquired(), Err(Cancelled));
        assert_eq!(oversized.acquired(), Err(Cancelled));
        drop(held);
        assert_eq!(budget.in_flight(), 0);
        assert_eq!(budget.acquire(10).map(|_| ()), Err(Cancelled));
    }

    #[test]
    fn a_charge_on_a_full_budget_returns_at_once_and_passes_the_cap() {
        let budget = MemoryBudget::new(100);
        let held = budget.acquire(100).unwrap();
        let charged = charged(&budget, 30);
        assert_eq!(charged.size(), 30);
        assert_eq!(budget.in_flight(), 130);
        drop(charged);
        assert_eq!(budget.in_flight(), 100);
        drop(held);
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn a_charge_on_a_cancelled_budget_returns_at_once_and_holds_until_dropped() {
        let budget = MemoryBudget::new(100);
        budget.cancel();
        let charged = charged(&budget, 40);
        assert_eq!(budget.in_flight(), 40);
        drop(charged);
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn an_ordinary_waiter_waits_while_charges_hold_the_budget_over_the_cap() {
        let budget = MemoryBudget::new(100);
        let first = charged(&budget, 80);
        let second = charged(&budget, 60);
        let waiter = waiter(&budget, 50);
        waiter.blocked();
        drop(first);
        // 60 + 50 is still over the cap.
        waiter.blocked();
        drop(second);
        assert_eq!(waiter.acquired(), Ok(()));
        waiter.release();
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn peak_is_the_most_held_through_acquire_and_charge() {
        let budget = MemoryBudget::new(20);
        assert_eq!(budget.peak(), 0);
        let acquired = budget.acquire(10).unwrap();
        assert_eq!(budget.peak(), 10);
        let charged = charged(&budget, 30);
        assert_eq!(budget.peak(), 40);
        drop(acquired);
        drop(charged);
        let later = budget.acquire(5).unwrap();
        assert_eq!(budget.peak(), 40);
        drop(later);
        assert_eq!(budget.peak(), 40);
    }

    #[test]
    fn is_cancelled_is_false_until_cancel_is_called() {
        let budget = MemoryBudget::new(100);
        assert!(!budget.is_cancelled());
        budget.cancel();
        assert!(budget.is_cancelled());
    }

    #[test]
    fn cap_is_the_cap_the_budget_was_made_with() {
        assert_eq!(MemoryBudget::new(100).cap(), 100);
    }

    #[test]
    fn the_floor_kept_archives_leave_free_is_one_gib() {
        // The literal is the point: the maintainer's value, written out so a slipped shift
        // in the constant is caught.
        assert_eq!(MIN_FREE_BUDGET, 1_073_741_824);
    }
}
