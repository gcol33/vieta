//! Cancellation for construction-time work.
//!
//! D22 puts interrupt and fuel in the evaluation context. Construction runs
//! below that context and can still be unbounded: a rewrite loop calling
//! [`Store::app`](crate::Store::app) in a tight loop builds terms faster than
//! anything above it can notice, and folding a large exact power is generative
//! work inside one call. A token installed on the store gives both a stopping
//! point.
//!
//! Cancellation is not a normalization choice and takes no part in the
//! equational theory of Layer A: a cancelled construction yields no term at all
//! rather than a differently normalized one.

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A flag one thread sets to stop construction on another.
///
/// Clones share the flag, so the thread driving a computation and the thread
/// handling an interrupt hold the same token.
#[derive(Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// A token that has not been cancelled.
    pub fn new() -> Self {
        CancelToken::default()
    }

    /// Set the flag. Every clone observes it.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Whether the flag has been set.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Construction stopped because the store's cancellation token was set.
///
/// The only way [`Store::app`](crate::Store::app) fails. Layer A is total
/// (`docs/layer-a.md` §8), so a term it cannot normalize is interned as
/// written rather than reported as an error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("term construction was cancelled")
    }
}

impl std::error::Error for Cancelled {}

#[cfg(test)]
mod tests {
    use super::CancelToken;

    #[test]
    fn a_fresh_token_is_not_cancelled() {
        assert!(!CancelToken::new().is_cancelled());
    }

    #[test]
    fn clones_share_the_flag() {
        let token = CancelToken::new();
        let other = token.clone();
        assert!(!other.is_cancelled());
        token.cancel();
        assert!(other.is_cancelled());
    }

    #[test]
    fn cancellation_crosses_threads() {
        let token = CancelToken::new();
        let other = token.clone();
        std::thread::spawn(move || other.cancel()).join().expect("thread ran");
        assert!(token.is_cancelled());
    }
}
