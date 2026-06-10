//! Platform-gated shared-state primitives for Gunmetal.
//!
//! Abstracts the difference between native (thread-safe) and WASM
//! (single-threaded) concurrency. All lock acquisition goes through
//! helper functions so callers don't need platform-specific code.
//!
//! - **Native:** `Arc<Mutex<T>>` / `Arc<RwLock<T>>` with poison recovery
//! - **WASM:** `Rc<RefCell<T>>` — zero-cost, single-threaded

// ── Native (multi-threaded) ─────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

    /// Shared mutable state requiring exclusive access.
    pub type SharedMut<T> = Arc<Mutex<T>>;

    /// Shared state supporting concurrent readers and exclusive writers.
    pub type SharedRead<T> = Arc<RwLock<T>>;

    /// Create a new shared-mutable container.
    pub fn new_shared_mut<T>(val: T) -> SharedMut<T> {
        Arc::new(Mutex::new(val))
    }

    /// Create a new shared-read container.
    pub fn new_shared_read<T>(val: T) -> SharedRead<T> {
        Arc::new(RwLock::new(val))
    }

    /// Acquire exclusive access. Recovers from poisoning (H5 fix).
    pub fn lock_mut<T>(s: &SharedMut<T>) -> MutexGuard<'_, T> {
        s.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Acquire shared read access. Recovers from poisoning.
    pub fn read_lock<T>(s: &SharedRead<T>) -> RwLockReadGuard<'_, T> {
        s.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Acquire exclusive write access. Recovers from poisoning.
    pub fn write_lock<T>(s: &SharedRead<T>) -> RwLockWriteGuard<'_, T> {
        s.write().unwrap_or_else(|e| e.into_inner())
    }
}

// ── WASM (single-threaded) ──────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod platform {
    use std::cell::{Ref, RefCell, RefMut};
    use std::rc::Rc;

    /// Shared mutable state (single-threaded, zero-cost).
    pub type SharedMut<T> = Rc<RefCell<T>>;

    /// Shared readable state (same as SharedMut on WASM — no thread contention).
    pub type SharedRead<T> = Rc<RefCell<T>>;

    /// Create a new shared-mutable container.
    pub fn new_shared_mut<T>(val: T) -> SharedMut<T> {
        Rc::new(RefCell::new(val))
    }

    /// Create a new shared-read container.
    pub fn new_shared_read<T>(val: T) -> SharedRead<T> {
        Rc::new(RefCell::new(val))
    }

    /// Acquire exclusive access (borrow_mut on WASM).
    pub fn lock_mut<T>(s: &SharedMut<T>) -> RefMut<'_, T> {
        s.borrow_mut()
    }

    /// Acquire shared read access (borrow on WASM).
    pub fn read_lock<T>(s: &SharedRead<T>) -> Ref<'_, T> {
        s.borrow()
    }

    /// Acquire exclusive write access (borrow_mut on WASM).
    pub fn write_lock<T>(s: &SharedRead<T>) -> RefMut<'_, T> {
        s.borrow_mut()
    }
}

pub use platform::*;

// ── Platform-conditional Send bound ─────────────────────────────────

/// `Send` on native, no-op on WASM.
///
/// Use as a bound on callbacks that are stored in shared state: on native,
/// `Gun` is `Send + Sync` so callbacks must be `Send`; on single-threaded
/// WASM, `JsValue`-capturing closures are accepted as-is.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}

/// `Send` on native, no-op on WASM (WASM variant).
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_mut_create_and_lock() {
        let s = new_shared_mut(42);
        {
            let mut guard = lock_mut(&s);
            *guard = 100;
        }
        assert_eq!(*lock_mut(&s), 100);
    }

    #[test]
    fn shared_read_create_and_read() {
        let s = new_shared_read(String::from("hello"));
        let guard = read_lock(&s);
        assert_eq!(&*guard, "hello");
    }

    #[test]
    fn shared_read_write_then_read() {
        let s = new_shared_read(vec![1, 2, 3]);
        {
            let mut guard = write_lock(&s);
            guard.push(4);
        }
        let guard = read_lock(&s);
        assert_eq!(&*guard, &[1, 2, 3, 4]);
    }

    #[test]
    fn shared_mut_clone_shares_state() {
        let s1 = new_shared_mut(0u32);
        let s2 = s1.clone();
        *lock_mut(&s1) = 42;
        assert_eq!(*lock_mut(&s2), 42);
    }

    #[test]
    fn shared_read_clone_shares_state() {
        let s1 = new_shared_read(0u32);
        let s2 = s1.clone();
        *write_lock(&s1) = 99;
        assert_eq!(*read_lock(&s2), 99);
    }

    #[test]
    fn multiple_readers_sequential() {
        let s = new_shared_read(String::from("data"));
        // On native this proves RwLock allows multiple readers;
        // on WASM it proves RefCell allows sequential borrows.
        let r1 = read_lock(&s);
        assert_eq!(&*r1, "data");
        drop(r1);
        let r2 = read_lock(&s);
        assert_eq!(&*r2, "data");
    }
}
