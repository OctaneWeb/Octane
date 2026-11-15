//! Graceful shutdown. Once it starts, listeners stop accepting, connections
//! waiting for a request close, and a connection answering one closes after
//! the response, which says `Connection: close`.
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

pub(crate) struct Shutdown {
    started: AtomicBool,
    // A slot per listener and connection, holding the waker of its task
    // while it waits. Each registers once rather than per wait, so waiting
    // for a request costs no lock shared with other connections.
    waiting: Mutex<Slab<Arc<Mutex<Option<Waker>>>>>,
}

impl Shutdown {
    pub(crate) fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            waiting: Mutex::new(Slab::new()),
        }
    }

    pub(crate) fn started(&self) -> bool {
        self.started.load(SeqCst)
    }

    /// Start shutting down, waking every listener and connection waiting
    pub(crate) fn start(&self) {
        self.started.store(true, SeqCst);
        for slot in lock(&self.waiting).iter() {
            if let Some(waker) = lock(slot).take() {
                waker.wake();
            }
        }
    }

    /// Register a task that waits on the shutdown, until the [`Watch`] drops
    pub(crate) fn watch(self: &Arc<Self>) -> Watch {
        let slot = Arc::new(Mutex::new(None));
        let key = lock(&self.waiting).insert(Arc::clone(&slot));
        Watch {
            shutdown: Arc::clone(self),
            key,
            slot,
        }
    }
}

/// A task's registration with [`Shutdown`]
pub(crate) struct Watch {
    shutdown: Arc<Shutdown>,
    key: usize,
    slot: Arc<Mutex<Option<Waker>>>,
}

impl Watch {
    pub(crate) fn started(&self) -> bool {
        self.shutdown.started()
    }

    /// Ready once the shutdown has started
    pub(crate) fn poll_started(&self, cx: &mut Context<'_>) -> Poll<()> {
        if self.started() {
            return Poll::Ready(());
        }
        {
            let mut waker = lock(&self.slot);
            if !waker.as_ref().map_or(false, |w| w.will_wake(cx.waker())) {
                *waker = Some(cx.waker().clone());
            }
        }
        // `start` may have taken the slot's old waker just before this one
        // went in, but then the flag it set first is visible here
        if self.started() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        lock(&self.shutdown.waiting).remove(self.key);
    }
}

// A lock that a panic elsewhere doesn't make unusable: what's under these
// is consistent after any single operation
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// Values kept by the key they were inserted at, which stays theirs until
// they're removed. Freed keys are reused, so it grows to the most entries
// held at once.
struct Slab<T> {
    entries: Vec<Option<T>>,
    free: Vec<usize>,
}

impl<T> Slab<T> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            free: Vec::new(),
        }
    }

    fn insert(&mut self, value: T) -> usize {
        match self.free.pop() {
            Some(key) => {
                self.entries[key] = Some(value);
                key
            }
            None => {
                self.entries.push(Some(value));
                self.entries.len() - 1
            }
        }
    }

    fn remove(&mut self, key: usize) {
        if self.entries.get_mut(key).and_then(Option::take).is_some() {
            self.free.push(key);
        }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;

    #[test]
    fn slab_reuses_freed_keys() {
        let mut slab = Slab::new();
        let (a, b) = (slab.insert('a'), slab.insert('b'));
        slab.remove(a);
        slab.remove(a);
        assert_eq!(slab.iter().collect::<Vec<_>>(), vec![&'b']);
        assert_eq!(slab.insert('c'), a);
        assert_ne!(a, b);
        assert_eq!(slab.entries.len(), 2);
    }

    #[tokio::test]
    async fn waiting_tasks_wake_when_it_starts() {
        let shutdown = Arc::new(Shutdown::new());
        let watch = shutdown.watch();
        let waiting = tokio::spawn(async move { poll_fn(|cx| watch.poll_started(cx)).await });
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        shutdown.start();
        waiting.await.unwrap();
        // the finished task dropped its watch
        assert_eq!(lock(&shutdown.waiting).iter().count(), 0);
    }
}
