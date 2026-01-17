use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// --- Global time budget (deadline) for Search ---
// Using AtomicU64 for lock-free performance (checked on every node)
// Stores milliseconds since UNIX_EPOCH, or 0 if no deadline is set
static DEADLINE_MS: AtomicU64 = AtomicU64::new(0);

/// Set a hard time budget for the ongoing Search. Passing 0 disables the budget.
pub fn set_time_budget_ms(ms: usize) {
    let deadline = if ms == 0 {
        0
    } else {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now_ms.saturating_add(ms as u64)
    };
    DEADLINE_MS.store(deadline, Ordering::Relaxed);
}

/// Clear any active time budget (Search will run to completion by depth).
pub(crate) fn clear_time_budget() {
    DEADLINE_MS.store(0, Ordering::Relaxed);
}

/// Check if time budget has expired. Lock-free for performance.
/// Called on every node during search, so must be extremely fast.
#[inline]
pub fn time_is_up() -> bool {
    let deadline = DEADLINE_MS.load(Ordering::Relaxed);
    if deadline == 0 {
        return false;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    now_ms >= deadline
}
