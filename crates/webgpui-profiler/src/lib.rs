//! CPU frame-timing profiler for webgpui.
//!
//! # Usage
//! ```ignore
//! let mut timer = FrameTimer::new(120);
//! loop {
//!     timer.begin_frame();
//!     // ... render ...
//!     timer.end_frame();
//!     if let Some(stats) = timer.stats() {
//!         println!("avg={:.2}ms p95={:.2}ms", stats.avg_ms, stats.p95_ms);
//!     }
//! }
//! ```

use std::cell::Cell;
use std::collections::VecDeque;
use std::time::Instant;

// ---------------------------------------------------------------------------
// FrameStats
// ---------------------------------------------------------------------------

/// Aggregated frame-time statistics over a rolling window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    /// Mean frame time in milliseconds.
    pub avg_ms: f64,
    /// 95th-percentile frame time in milliseconds.
    pub p95_ms: f64,
    /// Maximum frame time in the window in milliseconds.
    pub max_ms: f64,
    /// Number of samples in this measurement.
    pub sample_count: usize,
    /// Fraction of frames where GPU submission was skipped (`frames_skipped / frames_total`).
    ///
    /// This is the **P4_GPU_SKIP_RATIO** CI gate metric — a higher ratio means
    /// more idle frames are being elided successfully.  Ranges from `0.0`
    /// (every frame submits GPU work) to `1.0` (every frame is skipped).
    pub skip_ratio: f64,
}

impl std::fmt::Display for FrameStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "avg={:.2}ms p95={:.2}ms max={:.2}ms (n={}) skip={:.1}%",
            self.avg_ms,
            self.p95_ms,
            self.max_ms,
            self.sample_count,
            self.skip_ratio * 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// FrameTimer
// ---------------------------------------------------------------------------

/// Rolling-window frame-time tracker.
///
/// Records the wall-clock duration of each frame and computes aggregate
/// statistics (average, p95, max) over the last `window` frames.
pub struct FrameTimer {
    /// Samples in milliseconds, newest at the back.
    samples: VecDeque<f64>,
    /// Maximum number of samples to retain.
    window: usize,
    /// Timestamp of the last `begin_frame` call.
    frame_start: Option<Instant>,
    /// Cached result of the last `stats()` computation.
    /// Invalidated whenever a new sample is added or the timer is reset.
    cached_stats: Cell<Option<FrameStats>>,
    /// Total frames since the timer was created or last reset.
    frames_total: u64,
    /// Frames where GPU submission was skipped (P4 render-skip, see [`record_skip`][Self::record_skip]).
    frames_skipped: u64,
}

impl FrameTimer {
    /// Creates a new timer with a rolling window of `window` frames.
    pub fn new(window: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(window),
            window,
            frame_start: None,
            cached_stats: Cell::new(None),
            frames_total: 0,
            frames_skipped: 0,
        }
    }

    /// Records the start of a frame.  Should be called once per frame before
    /// any update or render work.
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    /// Records the end of a frame and stores the elapsed time.
    ///
    /// Returns `None` if `begin_frame` was not called first.
    pub fn end_frame(&mut self) -> Option<f64> {
        let start = self.frame_start.take()?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
        if self.samples.len() == self.window {
            self.samples.pop_front();
        }
        self.samples.push_back(elapsed_ms);
        self.cached_stats.set(None);
        self.frames_total += 1;
        Some(elapsed_ms)
    }

    /// Records a frame where GPU submission was skipped (P4 render-skip).
    ///
    /// Call this instead of (or after) [`end_frame`][Self::end_frame] on frames
    /// where the renderer decided not to submit GPU work because the
    /// `DirtyTracker` reported no dirty regions.
    ///
    /// The skip count feeds the `P4_GPU_SKIP_RATIO` metric in [`FrameStats::skip_ratio`].
    ///
    /// # Example
    ///
    /// ```
    /// use webgpui_profiler::FrameTimer;
    ///
    /// let mut t = FrameTimer::new(10);
    /// t.begin_frame();
    /// t.record_skip();
    /// t.end_frame();
    /// let stats = t.stats().unwrap();
    /// assert!(stats.skip_ratio > 0.0);
    /// ```
    pub fn record_skip(&mut self) {
        self.frames_skipped += 1;
        self.cached_stats.set(None);
    }

    /// Returns the total number of frames recorded since creation or last [`reset`][Self::reset].
    pub fn frames_total(&self) -> u64 {
        self.frames_total
    }

    /// Returns the number of frames where GPU submission was skipped since creation or last [`reset`][Self::reset].
    pub fn frames_skipped(&self) -> u64 {
        self.frames_skipped
    }

    /// Returns aggregated statistics if at least one sample is available.
    ///
    /// The result is cached after the first call and reused until a new sample
    /// is recorded (via [`FrameTimer::end_frame`]) or the timer is [`FrameTimer::reset`].
    pub fn stats(&self) -> Option<FrameStats> {
        if self.samples.is_empty() {
            return None;
        }
        if let Some(cached) = self.cached_stats.get() {
            return Some(cached);
        }
        let mut sorted: Vec<f64> = self.samples.iter().copied().collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        let avg_ms = sorted.iter().sum::<f64>() / n as f64;
        let p95_idx = ((n as f64 * 0.95) as usize).min(n - 1);
        let p95_ms = sorted[p95_idx];
        let max_ms = sorted[n - 1];
        let skip_ratio = if self.frames_total > 0 {
            self.frames_skipped as f64 / self.frames_total as f64
        } else {
            0.0
        };
        let stats = FrameStats {
            avg_ms,
            p95_ms,
            max_ms,
            sample_count: n,
            skip_ratio,
        };
        self.cached_stats.set(Some(stats));
        Some(stats)
    }

    /// Checks performance thresholds and logs warnings when targets are
    /// exceeded.
    ///
    /// * `avg_budget_ms`  – acceptable average frame time (e.g. 16.6 ms)
    /// * `p95_budget_ms`  – acceptable p95 frame time (e.g. 20.0 ms)
    pub fn check_thresholds(&self, avg_budget_ms: f64, p95_budget_ms: f64) {
        if let Some(stats) = self.stats() {
            if stats.avg_ms > avg_budget_ms {
                log::warn!(
                    "[profiler] avg frame time {:.2}ms exceeds budget {:.2}ms",
                    stats.avg_ms,
                    avg_budget_ms
                );
            }
            if stats.p95_ms > p95_budget_ms {
                log::warn!(
                    "[profiler] p95 frame time {:.2}ms exceeds budget {:.2}ms",
                    stats.p95_ms,
                    p95_budget_ms
                );
            }
        }
    }

    /// Clears all stored samples and resets skip counters.
    pub fn reset(&mut self) {
        self.samples.clear();
        self.frame_start = None;
        self.cached_stats.set(None);
        self.frames_total = 0;
        self.frames_skipped = 0;
    }
}

// ---------------------------------------------------------------------------
// SpanTimer – lightweight named section timer
// ---------------------------------------------------------------------------

/// Measures the elapsed time of a named code section.
///
/// ```ignore
/// let _span = SpanTimer::start("layout");
/// // ... layout work ...
/// // span is dropped here and logs the elapsed time
/// ```
pub struct SpanTimer {
    name: &'static str,
    start: Instant,
}

impl SpanTimer {
    /// Starts timing a named span.
    pub fn start(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }

    /// Returns elapsed milliseconds without stopping the timer.
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1_000.0
    }
}

impl Drop for SpanTimer {
    fn drop(&mut self) {
        log::trace!("[span] {} = {:.3}ms", self.name, self.elapsed_ms());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn timer_records_samples() {
        let mut t = FrameTimer::new(10);
        for _ in 0..5 {
            t.begin_frame();
            thread::sleep(Duration::from_millis(1));
            t.end_frame();
        }
        let stats = t.stats().unwrap();
        assert_eq!(stats.sample_count, 5);
        assert!(stats.avg_ms >= 1.0);
    }

    #[test]
    fn timer_rolling_window() {
        let mut t = FrameTimer::new(3);
        for _ in 0..5 {
            t.begin_frame();
            t.end_frame();
        }
        assert_eq!(t.stats().unwrap().sample_count, 3);
    }

    #[test]
    fn reset_clears_cached_stats() {
        let mut t = FrameTimer::new(10);
        t.begin_frame();
        t.end_frame();
        assert!(t.stats().is_some());
        t.reset();
        assert!(t.stats().is_none());
    }

    // P4_GPU_SKIP_RATIO metric tests.

    #[test]
    fn skip_ratio_zero_when_no_skips() {
        let mut t = FrameTimer::new(10);
        for _ in 0..5 {
            t.begin_frame();
            t.end_frame();
        }
        let stats = t.stats().unwrap();
        assert_eq!(
            stats.skip_ratio, 0.0,
            "no record_skip calls → ratio must be 0"
        );
    }

    #[test]
    fn skip_ratio_one_when_all_skipped() {
        let mut t = FrameTimer::new(10);
        for _ in 0..4 {
            t.begin_frame();
            t.record_skip();
            t.end_frame();
        }
        let stats = t.stats().unwrap();
        assert!(
            (stats.skip_ratio - 1.0).abs() < 1e-9,
            "all frames skipped → ratio must be 1.0, got {}",
            stats.skip_ratio
        );
    }

    #[test]
    fn skip_ratio_half_when_alternating() {
        let mut t = FrameTimer::new(20);
        for i in 0..10u32 {
            t.begin_frame();
            if i % 2 == 0 {
                t.record_skip();
            }
            t.end_frame();
        }
        let stats = t.stats().unwrap();
        assert!(
            (stats.skip_ratio - 0.5).abs() < 1e-9,
            "half frames skipped → ratio must be 0.5, got {}",
            stats.skip_ratio
        );
    }

    #[test]
    fn skip_ratio_reset_clears_counters() {
        let mut t = FrameTimer::new(10);
        t.begin_frame();
        t.record_skip();
        t.end_frame();
        assert!(t.stats().unwrap().skip_ratio > 0.0);
        t.reset();
        t.begin_frame();
        t.end_frame();
        let stats = t.stats().unwrap();
        assert_eq!(stats.skip_ratio, 0.0, "after reset, skip ratio must be 0");
    }

    #[test]
    fn frames_total_and_skipped_accessors() {
        let mut t = FrameTimer::new(10);
        for i in 0..6u32 {
            t.begin_frame();
            if i < 2 {
                t.record_skip();
            }
            t.end_frame();
        }
        assert_eq!(t.frames_total(), 6);
        assert_eq!(t.frames_skipped(), 2);
    }

    #[test]
    fn display_includes_skip_percentage() {
        let stats = FrameStats {
            avg_ms: 10.0,
            p95_ms: 15.0,
            max_ms: 20.0,
            sample_count: 5,
            skip_ratio: 0.25,
        };
        let s = stats.to_string();
        assert!(s.contains("skip="), "display must show skip ratio");
        assert!(
            s.contains("25.0%"),
            "display must show 25.0% for ratio 0.25"
        );
    }
}
