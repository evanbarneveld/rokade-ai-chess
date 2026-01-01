use std::sync::{Arc, Mutex, OnceLock};

// --- Lightweight info callback support to report progress while searching ---
// Include UCI hashfull (permill) so GUI can display hash usage
type InfoCb = dyn Fn(((usize, usize), (usize, usize)), i32, usize, Vec<((usize, usize), (usize, usize))>, u16)
+ Send
+ Sync
+ 'static;
static INFO_CB: OnceLock<Mutex<Option<Arc<InfoCb>>>> = OnceLock::new();

fn info_cb_cell() -> &'static Mutex<Option<Arc<InfoCb>>> {
    INFO_CB.get_or_init(|| Mutex::new(None))
}

pub fn set_info_callback(cb: Option<Arc<InfoCb>>) {
    let cell = info_cb_cell();
    let mut guard = cell.lock().unwrap();
    *guard = cb;
}

pub fn emit_info(
    from: (usize, usize),
    to: (usize, usize),
    score_cp: i32,
    depth_used: usize,
    pv: Vec<((usize, usize), (usize, usize))>,
    hashfull_permille: u16,
) {
    if let Some(cb) = info_cb_cell().lock().unwrap().as_ref().cloned() {
        (cb)(
            ((from.0, from.1), (to.0, to.1)),
            score_cp,
            depth_used,
            pv,
            hashfull_permille,
        )
    }
}
