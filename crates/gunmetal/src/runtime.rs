//! Runtime abstraction — spawn/sleep across platforms.
//!
//! Provides both sync and async variants:
//! - `spawn` / `sleep` — sync (thread-based on native, immediate/no-op on WASM)
//! - `spawn_async` / `sleep_async` — async (tokio on native, spawn_local/setTimeout on WASM)

use std::time::Duration;

// ── Sync spawn ─────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(f);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(f: F)
where
    F: FnOnce() + 'static,
{
    f();
}

// ── Sync sleep ─────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}

#[cfg(target_arch = "wasm32")]
pub fn sleep(_duration: Duration) {}

// ── Async spawn ────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_async<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_async<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

// ── Async sleep ────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_async(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep_async(duration: Duration) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let ms = duration.as_millis() as i32;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let global = js_sys::global();
        let set_timeout: js_sys::Function =
            js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
                .unwrap()
                .unchecked_into();
        set_timeout
            .call2(&JsValue::NULL, &resolve, &JsValue::from(ms))
            .unwrap();
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn spawn_executes_closure() {
        let flag = Arc::new(AtomicBool::new(false));
        let f = flag.clone();
        spawn(move || {
            f.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(50));
        assert!(flag.load(Ordering::SeqCst), "spawned closure should have executed");
    }

    #[test]
    fn sleep_does_not_panic() {
        sleep(Duration::from_millis(1));
    }

    #[test]
    fn spawn_can_access_shared_state() {
        let counter = Arc::new(std::sync::Mutex::new(0u32));
        let c = counter.clone();
        spawn(move || {
            *c.lock().unwrap() += 1;
        });
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn spawn_async_executes_future() {
        let flag = Arc::new(AtomicBool::new(false));
        let f = flag.clone();
        spawn_async(async move {
            f.store(true, Ordering::SeqCst);
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn sleep_async_completes() {
        let start = std::time::Instant::now();
        sleep_async(Duration::from_millis(50)).await;
        assert!(start.elapsed() >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn spawn_async_with_shared_state() {
        let counter = Arc::new(std::sync::Mutex::new(0u32));
        let c = counter.clone();
        spawn_async(async move {
            *c.lock().unwrap() += 1;
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(*counter.lock().unwrap(), 1);
    }
}
