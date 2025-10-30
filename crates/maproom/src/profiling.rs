//! Performance profiling utilities using puffin.
//!
//! This module provides profiling infrastructure for identifying performance bottlenecks.
//! Enable with the `profiling` feature flag.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crewchief_maproom::profiling::profile_scope;
//!
//! fn some_operation() {
//!     profile_scope!("some_operation");
//!     // ... operation code ...
//! }
//! ```
//!
//! # Conditional Compilation
//!
//! All profiling code is compiled out unless the `profiling` feature is enabled,
//! ensuring zero overhead in production builds.

/// Profile a scope with the given name.
///
/// This macro expands to `puffin::profile_scope!` when the `profiling` feature is enabled,
/// and to nothing otherwise.
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!($name);
    };
}

/// Profile a function with its name.
///
/// This macro expands to `puffin::profile_function!` when the `profiling` feature is enabled,
/// and to nothing otherwise.
#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
    };
}

/// Execute a closure with profiling enabled.
///
/// Returns the result of the closure. When profiling is disabled, this has zero overhead.
#[inline]
pub fn profile_operation<T, F>(#[allow(unused_variables)] name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    #[cfg(feature = "profiling")]
    {
        puffin::profile_scope!(name);
        f()
    }

    #[cfg(not(feature = "profiling"))]
    f()
}

/// Initialize puffin profiling.
///
/// Call this once at the start of your program to enable profiling data collection.
#[cfg(feature = "profiling")]
pub fn init_profiling() {
    puffin::set_scopes_on(true);
}

#[cfg(not(feature = "profiling"))]
pub fn init_profiling() {
    // No-op when profiling is disabled
}

/// Get profiling data for export.
///
/// Returns profiling frames that can be saved or sent to a viewer.
#[cfg(feature = "profiling")]
pub fn get_profiling_data() -> Vec<puffin::FrameData> {
    puffin::GlobalProfiler::lock().new_frames()
}

#[cfg(not(feature = "profiling"))]
pub fn get_profiling_data() -> Vec<()> {
    Vec::new()
}
