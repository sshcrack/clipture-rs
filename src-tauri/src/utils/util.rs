use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct AtomicDropGuard {
    b: Arc<AtomicBool>,
}

impl AtomicDropGuard {
    pub fn new(b: Arc<AtomicBool>) -> Self {
        b.store(true, Ordering::Relaxed);
        Self { b }
    }
}

impl Drop for AtomicDropGuard {
    fn drop(&mut self) {
        self.b.store(false, Ordering::Release);
    }
}
