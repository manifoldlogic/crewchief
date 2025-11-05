//! Progress tracking for indexing operations
//!
//! Provides real-time progress feedback during scan and indexing operations
//! with smart TTY detection and throttling to avoid output flooding.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Output mode for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Minimal, compact output (default)
    Minimal,
    /// Detailed, verbose output
    Verbose,
}

/// Tracks and displays progress for indexing operations
///
/// Thread-safe progress tracker that can be shared across threads.
/// Uses atomic counters for file/chunk counts and throttling to
/// prevent output flooding.
pub struct ProgressTracker {
    mode: OutputMode,
    is_tty: bool,
    start_time: Instant,
    total_files: Mutex<Option<usize>>,
    total_chunks: Mutex<Option<usize>>,
    processed_files: AtomicUsize,
    processed_chunks: AtomicUsize,
    last_update: Mutex<Option<Instant>>,
    last_percentage: AtomicUsize,
}

impl ProgressTracker {
    /// Create a new progress tracker
    ///
    /// # Arguments
    /// * `mode` - Output mode (Minimal or Verbose)
    ///
    /// # Examples
    /// ```
    /// use crewchief_maproom::progress::{ProgressTracker, OutputMode};
    ///
    /// let tracker = ProgressTracker::new(OutputMode::Minimal);
    /// ```
    pub fn new(mode: OutputMode) -> Self {
        // Detect if stdout is a TTY
        let is_tty = atty::is(atty::Stream::Stdout);

        Self {
            mode,
            is_tty,
            start_time: Instant::now(),
            total_files: Mutex::new(None),
            total_chunks: Mutex::new(None),
            processed_files: AtomicUsize::new(0),
            processed_chunks: AtomicUsize::new(0),
            last_update: Mutex::new(None),
            last_percentage: AtomicUsize::new(0),
        }
    }

    /// Set total file and chunk counts
    ///
    /// # Arguments
    /// * `files` - Total number of files to process
    /// * `chunks` - Optional total number of chunks (embeddings)
    pub fn set_totals(&self, files: usize, chunks: Option<usize>) {
        if let Ok(mut total_files) = self.total_files.lock() {
            *total_files = Some(files);
        }
        if let Ok(mut total_chunks) = self.total_chunks.lock() {
            *total_chunks = chunks;
        }
    }

    /// Update the count of processed files
    ///
    /// # Arguments
    /// * `count` - New count of processed files
    pub fn update_files(&self, count: usize) {
        self.processed_files.store(count, Ordering::Relaxed);
    }

    /// Update the count of processed chunks
    ///
    /// # Arguments
    /// * `count` - New count of processed chunks
    pub fn update_chunks(&self, count: usize) {
        self.processed_chunks.store(count, Ordering::Relaxed);
    }

    /// Check if progress should be printed
    ///
    /// Returns true if more than 200ms has elapsed since last print,
    /// preventing output flooding.
    ///
    /// The first call always returns true to allow initial progress display.
    pub fn should_print(&self) -> bool {
        if let Ok(mut last) = self.last_update.lock() {
            let now = Instant::now();

            match *last {
                None => {
                    // First call - allow print
                    *last = Some(now);
                    true
                }
                Some(last_time) => {
                    // Subsequent calls - check throttle
                    if now.duration_since(last_time) > Duration::from_millis(200) {
                        *last = Some(now);
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    /// Print current progress
    ///
    /// Format depends on TTY status and output mode:
    /// - TTY: Overwrites line with \r
    /// - Non-TTY: Prints new line every 10% progress
    pub fn print_progress(&self) {
        let files_processed = self.processed_files.load(Ordering::Relaxed);
        let chunks_processed = self.processed_chunks.load(Ordering::Relaxed);

        let total_files = self.total_files.lock().ok().and_then(|t| *t);
        let total_chunks = self.total_chunks.lock().ok().and_then(|t| *t);

        if self.is_tty {
            // TTY mode: overwrite line
            let mut output = String::new();

            if let Some(total) = total_files {
                if total > 0 {
                    let pct = self.percentage_files();
                    output.push_str(&format!(
                        "Processing: {}/{} files ({}%)",
                        files_processed, total, pct
                    ));
                }
            }

            if let Some(total) = total_chunks {
                if total > 0 {
                    let pct = self.percentage_chunks();
                    if !output.is_empty() {
                        output.push_str(" | ");
                    }
                    output.push_str(&format!(
                        "Embeddings: {}/{} ({}%)",
                        chunks_processed, total, pct
                    ));
                }
            }

            if !output.is_empty() {
                print!("\r{}", output);
                // Flush to ensure immediate display
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
        } else {
            // Non-TTY mode: print every 10% progress
            let current_pct = self.percentage_files();
            let last_pct = self.last_percentage.load(Ordering::Relaxed);

            if current_pct >= last_pct + 10 {
                self.last_percentage.store(current_pct, Ordering::Relaxed);

                if let Some(total) = total_files {
                    if total > 0 {
                        println!(
                            "Progress: {}% complete ({}/{} files)",
                            current_pct, files_processed, total
                        );
                    }
                }
            }
        }
    }

    /// Print final timing summary
    ///
    /// Prints completion message with total elapsed time.
    pub fn finish(&self) {
        let elapsed = self.start_time.elapsed();

        if self.is_tty {
            // Clear the progress line
            print!("\r");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }

        println!("\n✅ Completed in {:.1}s", elapsed.as_secs_f64());
    }

    /// Calculate percentage of files processed
    fn percentage_files(&self) -> usize {
        if let Ok(total_files) = self.total_files.lock() {
            if let Some(total) = *total_files {
                if total > 0 {
                    let processed = self.processed_files.load(Ordering::Relaxed);
                    return (processed * 100) / total;
                }
            }
        }
        0
    }

    /// Calculate percentage of chunks processed
    fn percentage_chunks(&self) -> usize {
        if let Ok(total_chunks) = self.total_chunks.lock() {
            if let Some(total) = *total_chunks {
                if total > 0 {
                    let processed = self.processed_chunks.load(Ordering::Relaxed);
                    return (processed * 100) / total;
                }
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_new_creates_tracker() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        assert_eq!(tracker.mode, OutputMode::Minimal);
        assert_eq!(tracker.processed_files.load(Ordering::Relaxed), 0);
        assert_eq!(tracker.processed_chunks.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_percentage_calculation() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        tracker.set_totals(100, None);
        tracker.update_files(50);
        assert_eq!(tracker.percentage_files(), 50);

        tracker.update_files(75);
        assert_eq!(tracker.percentage_files(), 75);
    }

    #[test]
    fn test_percentage_calculation_edge_cases() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);

        // Test with 1 file
        tracker.set_totals(1, None);
        tracker.update_files(1);
        assert_eq!(tracker.percentage_files(), 100);

        // Test with large numbers
        tracker.set_totals(10000, None);
        tracker.update_files(3750);
        assert_eq!(tracker.percentage_files(), 37);
    }

    #[test]
    fn test_zero_total_safe() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        tracker.set_totals(0, None);
        // Should not panic
        assert_eq!(tracker.percentage_files(), 0);
    }

    #[test]
    fn test_throttling() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);

        // First call should return true
        let first = tracker.should_print();
        assert!(first, "First call to should_print() should return true");

        // Immediate second call should return false (throttled)
        let second = tracker.should_print();
        assert!(!second, "Second immediate call should be throttled");

        // Sleep to exceed throttle threshold
        std::thread::sleep(Duration::from_millis(300));

        // Should allow print now
        let third = tracker.should_print();
        assert!(third, "After sleep, should_print() should return true again");
    }

    #[test]
    fn test_throttling_timing() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);

        // Reset by calling once
        tracker.should_print();

        // Should be throttled
        assert!(!tracker.should_print());

        // Wait 250ms (> 200ms threshold)
        std::thread::sleep(Duration::from_millis(250));

        // Should allow print now
        assert!(tracker.should_print());
    }

    #[test]
    fn test_concurrent_updates() {
        let tracker = Arc::new(ProgressTracker::new(OutputMode::Minimal));
        tracker.set_totals(1000, None);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let t = Arc::clone(&tracker);
                thread::spawn(move || {
                    for i in 0..100 {
                        t.update_files((i + 1) * 10);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // All threads completed - verify tracker is in valid state
        let final_count = tracker.processed_files.load(Ordering::Relaxed);
        assert!(final_count <= 1000);
    }

    #[test]
    fn test_output_mode_minimal() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        assert_eq!(tracker.mode, OutputMode::Minimal);
    }

    #[test]
    fn test_output_mode_verbose() {
        let tracker = ProgressTracker::new(OutputMode::Verbose);
        assert_eq!(tracker.mode, OutputMode::Verbose);
    }

    #[test]
    fn test_set_totals_updates() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);

        tracker.set_totals(100, Some(500));

        {
            let files = tracker.total_files.lock().unwrap();
            assert_eq!(*files, Some(100));
        }

        {
            let chunks = tracker.total_chunks.lock().unwrap();
            assert_eq!(*chunks, Some(500));
        }
    }

    #[test]
    fn test_chunks_percentage() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        tracker.set_totals(100, Some(1000));
        tracker.update_chunks(250);
        assert_eq!(tracker.percentage_chunks(), 25);
    }
}
