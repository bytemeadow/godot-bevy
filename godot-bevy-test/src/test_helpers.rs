//! Ergonomic helpers for writing async integration tests

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

/// Helper to create a shared counter that can be used in systems
/// Clone is cheap (just an Arc clone), so it works in Fn closures
#[derive(Clone, Default)]
pub struct Counter(Arc<AtomicU32>);

impl Counter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU32::new(0)))
    }

    pub fn get(&self) -> u32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn increment(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
