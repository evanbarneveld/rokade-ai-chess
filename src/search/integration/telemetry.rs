use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// --- Simple global telemetry (nodes visited) for UCI info reporting ---
static NODE_COUNT: OnceLock<AtomicU64> = OnceLock::new();


#[inline]
fn node_count_cell() -> &'static AtomicU64 {
    NODE_COUNT.get_or_init(|| AtomicU64::new(0))
}

#[inline]
pub(crate) fn reset_search_telemetry() {
    node_count_cell().store(0, Ordering::Relaxed);
}

#[inline]
pub(crate) fn get_nodes() -> u64 {
    node_count_cell().load(Ordering::Relaxed)
}

#[inline]
pub fn bump_node() {
    let _ = node_count_cell().fetch_add(1, Ordering::Relaxed);
}
