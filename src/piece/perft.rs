use crate::search::core::advanced_search::{find_all_valid_moves, find_all_valid_moves_into_perft, PerftMove};
use crate::search::state::zobrist::compute_zobrist;
use crate::state::game_state::GameState;
use std::cell::RefCell;
use std::collections::HashMap;
use rayon::prelude::*;

thread_local! {
    // Perft transposition table: key = (zobrist-like state hash, depth), value = node count
    static PERFT_TT: RefCell<HashMap<(u64, u32), u64>> = RefCell::new(HashMap::new());
}

#[inline]
fn perft_state_key(gs: &GameState) -> u64 {
    // Base Zobrist from board + side to move
    let key = compute_zobrist(gs.board(), gs.active_color());

    // Mix in castling rights (4 bits) and en-passant file (0..8; 0 = none, 1..8 = file a..h)
    let cr = gs.castling_rights();
    let mut cr_bits: u8 = 0;
    // Order: KQkq -> bits 0..3
    // Accessors are exposed on CastlingRights
    if cr.white_kingside() { cr_bits |= 1 << 0; }
    if cr.white_queenside() { cr_bits |= 1 << 1; }
    if cr.black_kingside() { cr_bits |= 1 << 2; }
    if cr.black_queenside() { cr_bits |= 1 << 3; }

    let ep_file: u64 = if let Some((_, file)) = gs.en_passant_target() { (file as u64) + 1 } else { 0 };

    // Simple mix (SplitMix-inspired constants) to reduce collisions
    let mix1 = (cr_bits as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mix2 = ep_file.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    key ^ mix1 ^ mix2
}

#[inline]
pub fn perft_count(gs: &GameState, depth: u32) -> u64 {
    // Keep public API immutable; use a single mutable copy at root and recurse with make/unmake
    // Clear perft TT for a fresh top-level invocation to avoid cross-call growth
    PERFT_TT.with(|tt| tt.borrow_mut().clear());
    let mut work = *gs;
    perft_count_mut(&mut work, depth)
}

#[inline]
fn perft_count_mut(gs: &mut GameState, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    // Probe perft TT
    let k = perft_state_key(gs);
    if let Some(hit) = PERFT_TT.with(|tt| tt.borrow().get(&(k, depth)).cloned()) {
        return hit;
    }

    // Reuse a local buffer for move generation in this frame
    let mut moves_buf: Vec<PerftMove> = Vec::with_capacity(64);

    // Fill buffer using existing generator (which takes Vec)
    find_all_valid_moves_into_perft(gs, &mut moves_buf);

    if depth == 1 {
        return moves_buf.len() as u64;
    }

    let mut nodes: u64 = 0;
    for mv in moves_buf.into_iter() {
        let undo = gs.make_move_fast(mv.from, mv.to, mv.promo);
        nodes += perft_count_mut(gs, depth - 1);
        gs.unmake_move_fast(undo);
    }
    // Store in TT
    PERFT_TT.with(|tt| {
        tt.borrow_mut().insert((k, depth), nodes);
    });
    nodes
}

#[allow(dead_code)]
pub fn perft_divide(gs: &GameState, depth: u32) -> Vec<(String, u64)> {
    let mut result = Vec::new();
    if depth == 0 {
        return result;
    }
    // Ensure a clean TT for divide as well (not strictly necessary but keeps counts isolated)
    PERFT_TT.with(|tt| tt.borrow_mut().clear());
    let mut work = *gs;
    let moves = find_all_valid_moves(&mut work);
    for (from, to, promo) in moves {
        let u = work.make_move_fast(from, to, promo);
        let count = perft_count_mut(&mut work, depth - 1);
        work.unmake_move_fast(u);
        // format move as coordinate string
        let s = if let Some(pc) = promo {
            format!(
                "{}{}{}{}{}",
                (b'a' + from.1 as u8) as char,
                (b'1' + from.0 as u8) as char,
                (b'a' + to.1 as u8) as char,
                (b'1' + to.0 as u8) as char,
                pc
            )
        } else {
            format!(
                "{}{}{}{}",
                (b'a' + from.1 as u8) as char,
                (b'1' + from.0 as u8) as char,
                (b'a' + to.1 as u8) as char,
                (b'1' + to.0 as u8) as char
            )
        };
        result.push((s, count));
    }
    result
}

/// Parallel perft at root: splits root moves across threads and sums child node counts.
/// Default single-threaded `perft_count` remains for determinism; this is an opt-in helper.
pub fn perft_count_parallel(gs: &GameState, depth: u32) -> u64 {
    if depth == 0 { return 1; }

    // Build root move list
    let mut root_moves: Vec<PerftMove> = Vec::with_capacity(64);
    find_all_valid_moves_into_perft(gs, &mut root_moves);

    if depth == 1 {
        return root_moves.len() as u64;
    }

    // Clear TT in the main thread; each worker has its own thread-local map as well
    PERFT_TT.with(|tt| tt.borrow_mut().clear());

    // Use a copy of GameState per root branch to avoid sharing mutable state across threads
    root_moves
        .par_iter()
        .map(|mv| {
            // Each thread uses its own thread-local TT; clear it to avoid cross-branch pollution
            PERFT_TT.with(|tt| tt.borrow_mut().clear());
            let mut local = *gs;
            let u = local.make_move_fast(mv.from, mv.to, mv.promo);
            let nodes = perft_count_mut(&mut local, depth - 1);
            local.unmake_move_fast(u);
            nodes
        })
        .sum()
}
