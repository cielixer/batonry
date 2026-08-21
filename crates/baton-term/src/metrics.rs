//! Render-path instrumentation. **Not in the original `iced_term`.**
//!
//! Everything here is an atomic counter, so the render path takes no locks.
//! Move this behind a `metrics` feature (or delete it) once the numbers stop
//! being interesting.

use std::sync::atomic::{
    AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed,
};
use std::sync::Mutex;

/// Whether to batch glyph runs. Global so one binary can measure both the
/// as-is and the batched path.
static BATCHED: AtomicBool = AtomicBool::new(false);
/// Damage-based cache invalidation. Off means the original behaviour:
/// `cache.clear()` on every command.
static DAMAGE_AWARE: AtomicBool = AtomicBool::new(false);
/// Skip the full `Grid<Cell>` clone when nothing was damaged. The original
/// clones on every sync.
static SKIP_GRID_CLONE: AtomicBool = AtomicBool::new(false);

pub fn set_batched(v: bool) {
    BATCHED.store(v, Relaxed);
}
pub fn batched() -> bool {
    BATCHED.load(Relaxed)
}
pub fn set_damage_aware(v: bool) {
    DAMAGE_AWARE.store(v, Relaxed);
}
pub fn damage_aware() -> bool {
    DAMAGE_AWARE.load(Relaxed)
}
pub fn set_skip_grid_clone(v: bool) {
    SKIP_GRID_CLONE.store(v, Relaxed);
}
pub fn skip_grid_clone() -> bool {
    SKIP_GRID_CLONE.load(Relaxed)
}

/// `Widget::draw` calls. iced redraws the whole window, so this is
/// frames x panes.
pub static DRAW_CALLS: AtomicU64 = AtomicU64::new(0);
/// Of those, how many actually rebuilt geometry (cache misses).
pub static GEOM_BUILDS: AtomicU64 = AtomicU64::new(0);
/// Total time spent rebuilding geometry, in nanoseconds.
pub static GEOM_NS: AtomicU64 = AtomicU64::new(0);
/// `fill_text` calls. This is how you see whether batching actually helps.
pub static TEXT_OPS: AtomicU64 = AtomicU64::new(0);
/// Background quads after run-length merging.
pub static QUAD_OPS: AtomicU64 = AtomicU64::new(0);
/// How many times `Backend::sync` cloned the grid.
pub static GRID_CLONES: AtomicU64 = AtomicU64::new(0);
/// **Healthy.** Wakeups merged at the source. Higher is better: each one is
/// a UI turn we did not have to spend.
pub static WAKEUPS_COALESCED: AtomicU64 = AtomicU64::new(0);
/// **Data loss. Must stay 0; a test asserts it.**
///
/// Non-wakeup events we threw away. A lost `Exit` leaves a pane that never
/// learns its process died; a lost `PtyWrite` swallows a DA/DSR reply.
///
/// **Nothing increments this today** -- the unbounded path makes dropping
/// structurally impossible. It is a tripwire for anyone who reintroduces a
/// bounded send. `tests/deadlock.rs` asserts it is 0.
pub static EVENTS_DROPPED: AtomicU64 = AtomicU64::new(0);
/// Sends that failed because the receiver was gone. **Not data loss** --
/// this is how a pane closes.
pub static CHANNEL_CLOSED: AtomicU64 = AtomicU64::new(0);
/// Events currently sitting in the unbounded queue.
pub static EVENTS_QUEUED: AtomicU64 = AtomicU64::new(0);
/// High-water mark of the above. **This is the evidence that the unbounded
/// path is actually safe** -- it has to stay small.
pub static EVENTS_QUEUE_MAX: AtomicU64 = AtomicU64::new(0);
/// Backend commands processed.
pub static COMMANDS: AtomicU64 = AtomicU64::new(0);
/// Cell count seen on the last draw (cols x rows).
pub static CELLS: AtomicUsize = AtomicUsize::new(0);

/// Geometry rebuild samples, in nanoseconds, kept so we can take
/// percentiles.
static SAMPLES: Mutex<Vec<u32>> = Mutex::new(Vec::new());

pub fn record_geom(ns: u64) {
    GEOM_BUILDS.fetch_add(1, Relaxed);
    GEOM_NS.fetch_add(ns, Relaxed);
    if let Ok(mut s) = SAMPLES.lock() {
        // Cap the sample buffer: under a large load an unbounded Vec would
        // itself distort the measurement.
        if s.len() < 200_000 {
            s.push(ns.min(u32::MAX as u64) as u32);
        }
    }
}

pub fn reset() {
    for c in [
        &DRAW_CALLS,
        &GEOM_BUILDS,
        &GEOM_NS,
        &TEXT_OPS,
        &QUAD_OPS,
        &GRID_CLONES,
        &COMMANDS,
        &WAKEUPS_COALESCED,
        &EVENTS_DROPPED,
        &CHANNEL_CLOSED,
        &EVENTS_QUEUED,
        &EVENTS_QUEUE_MAX,
    ] {
        c.store(0, Relaxed);
    }
    if let Ok(mut s) = SAMPLES.lock() {
        s.clear();
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Percentiles {
    pub n: usize,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub mean_us: f64,
}

pub fn percentiles() -> Percentiles {
    let Ok(mut s) = SAMPLES.lock() else {
        return Percentiles::default();
    };
    if s.is_empty() {
        return Percentiles::default();
    }
    s.sort_unstable();
    let at = |q: f64| -> f64 {
        let i = ((s.len() as f64 - 1.0) * q).round() as usize;
        s[i] as f64 / 1000.0
    };
    Percentiles {
        n: s.len(),
        p50_us: at(0.50),
        p95_us: at(0.95),
        p99_us: at(0.99),
        max_us: *s.last().unwrap() as f64 / 1000.0,
        mean_us: s.iter().map(|v| *v as f64).sum::<f64>()
            / s.len() as f64
            / 1000.0,
    }
}
