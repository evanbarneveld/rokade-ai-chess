use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicI32, Ordering};

// --- Lightweight info callback support to report progress while searching ---
// Include UCI hashfull (permill) so GUI can display hash usage
type InfoCb = dyn Fn(((usize, usize), (usize, usize), Option<char>), i32, usize, Vec<((usize, usize), (usize, usize), Option<char>)>, u16)
+ Send
+ Sync
+ 'static;
static INFO_CB: OnceLock<Mutex<Option<Arc<InfoCb>>>> = OnceLock::new();

// Cache the last known reasonable evaluation score to avoid displaying extreme values
static LAST_EVAL_SCORE: AtomicI32 = AtomicI32::new(0);

// Threshold for detecting extreme/uninitialized scores (close to MIN/MAX_EVAL_VALUE)
const EXTREME_SCORE_THRESHOLD: i32 = 100_000_000;

fn info_cb_cell() -> &'static Mutex<Option<Arc<InfoCb>>> {
    INFO_CB.get_or_init(|| Mutex::new(None))
}

pub fn set_info_callback(cb: Option<Arc<InfoCb>>) {
    let cell = info_cb_cell();
    let mut guard = cell.lock().unwrap();
    *guard = cb;
}

/// Check if a score appears to be an extreme/uninitialized value
#[inline]
fn is_extreme_score(score: i32) -> bool {
    score.abs() > EXTREME_SCORE_THRESHOLD
}

/// Get the last cached evaluation score
pub fn get_last_eval_score() -> i32 {
    LAST_EVAL_SCORE.load(Ordering::Relaxed)
}

/// Set the cached evaluation score
pub fn set_last_eval_score(score: i32) {
    if !is_extreme_score(score) {
        LAST_EVAL_SCORE.store(score, Ordering::Relaxed);
    }
}

pub fn emit_info(
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score_cp: i32,
    depth_used: usize,
    pv: Vec<((usize, usize), (usize, usize), Option<char>)>,
    hashfull_permille: u16,
) {
    // If the score is extreme, use the cached last known score instead
    let display_score = if is_extreme_score(score_cp) {
        get_last_eval_score()
    } else {
        // Cache this reasonable score for future use
        set_last_eval_score(score_cp);
        score_cp
    };

    if let Some(cb) = info_cb_cell().lock().unwrap().as_ref().cloned() {
        (cb)(
            ((from.0, from.1), (to.0, to.1), promo),
            display_score,
            depth_used,
            pv,
            hashfull_permille,
        )
    }
}
