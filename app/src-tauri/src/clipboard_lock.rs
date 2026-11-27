//! One clipboard round trip at a time. Reading the selection (save, ⌘C,
//! restore) and Replace (save, ⌘V, restore 300 ms later) both borrow the
//! user's clipboard; two at once would save each other's temporary content
//! as "the user's clipboard" and restore the wrong thing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

static BUSY: AtomicBool = AtomicBool::new(false);

/// Held while the clipboard is borrowed; released on drop. Can be moved to
/// the thread that finishes the round trip.
#[derive(Debug)]
pub struct ClipboardLock(());

impl ClipboardLock {
    /// Waits until no other round trip holds the clipboard.
    pub fn acquire() -> Self {
        loop {
            if let Some(lock) = Self::try_acquire() {
                return lock;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn try_acquire() -> Option<Self> {
        BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| ClipboardLock(()))
    }
}

impl Drop for ClipboardLock {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test: the lock is global and tests run in parallel.
    #[test]
    fn one_holder_at_a_time_across_threads() {
        let first = ClipboardLock::try_acquire().expect("free at start");
        assert!(ClipboardLock::try_acquire().is_none());
        drop(first);
        let lock = ClipboardLock::acquire();
        assert!(ClipboardLock::try_acquire().is_none());
        std::thread::spawn(move || drop(lock))
            .join()
            .expect("thread");
        assert!(ClipboardLock::try_acquire().is_some());
    }
}
