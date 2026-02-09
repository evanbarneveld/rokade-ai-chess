use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::search::state::tt::{pow2_for_hash_mb, TranspositionTable};

const DETERMINISTIC_TIME_BONUS_PCT: usize = 50;

pub type InfoCb = dyn Fn(
    ((usize, usize), (usize, usize), Option<char>),
    i32,
    usize,
    Vec<((usize, usize), (usize, usize), Option<char>)>,
    u16,
) + Send
    + Sync
    + 'static;

pub struct SearchContext {
    deterministic: AtomicBool,
    parallel_search: AtomicBool,
    order_book_enabled: AtomicBool,
    start_ms: AtomicU64,
    deadline_ms: AtomicU64,
    node_count: AtomicU64,
    info_cb: Mutex<Option<Arc<InfoCb>>>,
    tt: Arc<TranspositionTable>,
}

impl std::fmt::Debug for SearchContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchContext")
            .field("deterministic", &self.is_deterministic())
            .field("parallel_search", &self.is_parallel_search())
            .field("order_book_enabled", &self.get_order_book_enabled())
            .field("deadline_ms", &self.deadline_ms.load(Ordering::Relaxed))
            .field("node_count", &self.node_count.load(Ordering::Relaxed))
            .finish()
    }
}

impl SearchContext {
    pub fn new() -> Self {
        Self {
            deterministic: AtomicBool::new(false),
            parallel_search: AtomicBool::new(true),
            order_book_enabled: AtomicBool::new(true),
            start_ms: AtomicU64::new(0),
            deadline_ms: AtomicU64::new(0),
            node_count: AtomicU64::new(0),
            info_cb: Mutex::new(None),
            tt: Arc::new(TranspositionTable::new_with_default_size()),
        }
    }

    pub fn new_with_hash_mb(hash_mb: usize) -> Self {
        let pow2 = pow2_for_hash_mb(hash_mb);
        Self {
            deterministic: AtomicBool::new(false),
            parallel_search: AtomicBool::new(true),
            order_book_enabled: AtomicBool::new(true),
            start_ms: AtomicU64::new(0),
            deadline_ms: AtomicU64::new(0),
            node_count: AtomicU64::new(0),
            info_cb: Mutex::new(None),
            tt: Arc::new(TranspositionTable::with_capacity_pow2(pow2)),
        }
    }

    /// Create a helper context for Lazy SMP that shares the TT with another context.
    /// Helper contexts have parallel search disabled (to avoid nested parallelism)
    /// and opening book disabled.
    pub fn new_smp_helper(shared_tt: Arc<TranspositionTable>) -> Self {
        Self {
            deterministic: AtomicBool::new(false),
            parallel_search: AtomicBool::new(false), // Disable nested parallelism
            order_book_enabled: AtomicBool::new(false), // Skip book in helpers
            start_ms: AtomicU64::new(0),
            deadline_ms: AtomicU64::new(0),
            node_count: AtomicU64::new(0),
            info_cb: Mutex::new(None),
            tt: shared_tt,
        }
    }

    /// Get the shared TT Arc for creating helper contexts
    pub fn shared_tt(&self) -> Arc<TranspositionTable> {
        Arc::clone(&self.tt)
    }

    #[inline]
    pub fn set_deterministic(&self, on: bool) {
        self.deterministic.store(on, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_deterministic(&self) -> bool {
        self.deterministic.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_parallel_search(&self, on: bool) {
        self.parallel_search.store(on, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_parallel_search(&self) -> bool {
        self.parallel_search.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_order_book_enabled(&self, on: bool) {
        self.order_book_enabled.store(on, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_order_book_enabled(&self) -> bool {
        self.order_book_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_time_budget_ms(&self, ms: usize) {
        let ms = if ms > 0 && self.is_deterministic() {
            ms.saturating_add(ms.saturating_mul(DETERMINISTIC_TIME_BONUS_PCT) / 100)
        } else {
            ms
        };
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.start_ms.store(now_ms, Ordering::Relaxed);
        let deadline = now_ms.saturating_add(ms as u64);
        self.deadline_ms.store(deadline, Ordering::Relaxed);
    }

    #[inline]
    pub fn clear_time_budget(&self) {
        self.start_ms.store(0, Ordering::Relaxed);
        self.deadline_ms.store(0, Ordering::Relaxed);
    }

    #[inline]
    pub fn time_is_up(&self) -> bool {
        let deadline = self.deadline_ms.load(Ordering::Relaxed);
        if deadline == 0 {
            return false;
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        now_ms >= deadline
    }

    #[inline]
    pub fn time_budget_ms(&self) -> usize {
        let start = self.start_ms.load(Ordering::Relaxed);
        let deadline = self.deadline_ms.load(Ordering::Relaxed);
        if start == 0 || deadline == 0 || deadline < start {
            return 0;
        }
        (deadline - start) as usize
    }

    #[inline]
    pub fn time_remaining_ms(&self) -> usize {
        let deadline = self.deadline_ms.load(Ordering::Relaxed);
        if deadline == 0 {
            return 0;
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        if now_ms >= deadline { 0 } else { (deadline - now_ms) as usize }
    }

    #[inline]
    pub fn extend_time_budget_ms(&self, add_ms: usize) {
        if add_ms == 0 {
            return;
        }
        let deadline = self.deadline_ms.load(Ordering::Relaxed);
        if deadline == 0 {
            return;
        }
        let new_deadline = deadline.saturating_add(add_ms as u64);
        self.deadline_ms.store(new_deadline, Ordering::Relaxed);
    }

    #[inline]
    pub fn reset_search_telemetry(&self) {
        self.node_count.store(0, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_nodes(&self) -> u64 {
        self.node_count.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn bump_node(&self) {
        let _ = self.node_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_info_callback(&self, cb: Option<Arc<InfoCb>>) {
        let mut guard = self.info_cb.lock().unwrap();
        *guard = cb;
    }

    #[inline]
    pub fn emit_info(
        &self,
        from: (usize, usize),
        to: (usize, usize),
        promo: Option<char>,
        score_cp: i32,
        depth_used: usize,
        pv: Vec<((usize, usize), (usize, usize), Option<char>)>,
        hashfull_permille: u16,
    ) {
        if let Some(cb) = self.info_cb.lock().unwrap().as_ref().cloned() {
            (cb)(
                ((from.0, from.1), (to.0, to.1), promo),
                score_cp,
                depth_used,
                pv,
                hashfull_permille,
            )
        }
    }

    #[inline]
    pub fn tt(&self) -> &TranspositionTable {
        &self.tt
    }
}

impl Default for SearchContext {
    fn default() -> Self {
        Self::new()
    }
}
