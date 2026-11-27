//! Lookup ids: each hotkey press starts a new lookup and makes older ones
//! stale, so their late deltas are dropped.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Sessions(AtomicU64);

impl Sessions {
    pub fn begin(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn is_current(&self, id: u64) -> bool {
        self.0.load(Ordering::SeqCst) == id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_current() {
        let s = Sessions::default();
        let a = s.begin();
        assert!(s.is_current(a));
    }

    #[test]
    fn stale_session_is_not_current() {
        let s = Sessions::default();
        let a = s.begin();
        let b = s.begin();
        assert!(!s.is_current(a));
        assert!(s.is_current(b));
    }
}
