use std::sync::{atomic::AtomicBool, Arc};

pub struct AtomicDropGuard {
    b: Arc<AtomicBool>
}

impl AtomicDropGuard {
    pub fn new(b: Arc<AtomicBool>) -> Self {
        b.store(true, std::sync::atomic::Ordering::SeqCst);
        Self { b}
    }
}

impl Drop for AtomicDropGuard {
    fn drop(&mut self) {
        self.b.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}