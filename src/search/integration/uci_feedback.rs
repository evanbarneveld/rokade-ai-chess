use std::sync::Arc;
use crate::search::context::{InfoCb, SearchContext};

// --- Lightweight info callback support to report progress while searching ---
// Include UCI hashfull (permill) so GUI can display hash usage
pub fn set_info_callback(ctx: &SearchContext, cb: Option<Arc<InfoCb>>) {
    ctx.set_info_callback(cb);
}

pub fn emit_info(
    ctx: &SearchContext,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score_cp: i32,
    depth_used: usize,
    pv: Vec<((usize, usize), (usize, usize), Option<char>)>,
    hashfull_permille: u16,
) {
    ctx.emit_info(from, to, promo, score_cp, depth_used, pv, hashfull_permille);
}
