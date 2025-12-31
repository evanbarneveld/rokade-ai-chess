use std::sync::OnceLock;
use rayon::ThreadPoolBuilder;

// Threading (Rayon) defaults
const RAYON_DEFAULT_THREADS: usize = 12; // default worker threads if env var is not set
const RAYON_STACK_BYTES: usize = 32 * 1024 * 1024; // per-thread stack size

pub fn init_rayon_pool_if_needed() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // Prefer 12 threads by default; allow env override via RAYON_NUM_THREADS
        let default_threads = RAYON_DEFAULT_THREADS;
        let num_threads = std::env::var("RAYON_NUM_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(default_threads);
        // Increase worker thread stack size to avoid stack overflows in deep searches.
        let stack_bytes: usize = RAYON_STACK_BYTES;
        let _ = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .stack_size(stack_bytes)
            .build_global();
    });
}
