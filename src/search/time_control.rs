use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// --- Global time budget (deadline) for Search ---
static DEADLINE_CELL: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

#[inline]
fn deadline_cell() -> &'static Mutex<Option<Instant>> {
    DEADLINE_CELL.get_or_init(|| Mutex::new(None))
}


/// Set a hard time budget for the ongoing Search. Passing 0 disables the budget.
pub(crate) fn set_time_budget_ms(ms: usize) {
    let mut guard = deadline_cell().lock().unwrap();
    if ms == 0 {
        *guard = None;
    } else {
        *guard = Some(Instant::now() + std::time::Duration::from_millis(ms as u64));
    }
}

/// Clear any active time budget (Search will run to completion by depth).
pub(crate) fn clear_time_budget() {
    let mut guard = deadline_cell().lock().unwrap();
    *guard = None;
}

#[inline]
pub fn time_is_up() -> bool {
    if let Some(dl) = *deadline_cell().lock().unwrap() {
        Instant::now() >= dl
    } else {
        false
    }
}
