use rand::{rng, Rng};
use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::board::evaluator::evaluate_position;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::search::zobrist::compute_zobrist;
use crate::search::tt::{TranspositionTable, Bound, encode_move, decode_move, to_tt_score, from_tt_score, MATE_VALUE};
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};

// History and Killer tables for move ordering
struct SearchHeuristics {
    // history[side][from][to] -> score
    history: [[[i32; 64]; 64]; 2],
    // two killer moves per ply, stored as (from*8+to)
    killers: Vec<[i16; 2]>,
}

impl SearchHeuristics {
    fn new(max_ply: usize) -> Self {
        SearchHeuristics {
            history: [[[0; 64]; 64]; 2],
            killers: vec![[ -1, -1 ]; max_ply.max(64)],
        }
    }
    #[inline]
    fn idx_side(side: Color) -> usize { if let Color::White = side { 0 } else { 1 } }
    #[inline]
    fn flat(from: (usize, usize), to: (usize, usize)) -> i16 { ((from.0*8 + from.1)*8 + (to.0*8 + to.1)) as i16 }

    fn add_killer(&mut self, ply: usize, from: (usize, usize), to: (usize, usize)) {
        if ply >= self.killers.len() { return; }
        let m = Self::flat(from, to);
        let k = &mut self.killers[ply];
        if k[0] != m {
            k[1] = k[0];
            k[0] = m;
        }
    }
    fn is_killer(&self, ply: usize, from: (usize, usize), to: (usize, usize)) -> bool {
        if ply >= self.killers.len() { return false; }
        let m = Self::flat(from, to);
        let k = self.killers[ply];
        k[0] == m || k[1] == m
    }
    fn add_history(&mut self, side: Color, from: (usize, usize), to: (usize, usize), bonus: i32) {
        let s = Self::idx_side(side);
        let f = from.0*8 + from.1; let t = to.0*8 + to.1;
        let entry = &mut self.history[s][f][t];
        *entry += bonus;
        // cap to avoid runaway values
        let cap = 1_000_000;
        if *entry > cap { *entry = cap; }
        if *entry < -cap { *entry = -cap; }
    }
    fn history_score(&self, side: Color, from: (usize, usize), to: (usize, usize)) -> i32 {
        let s = Self::idx_side(side);
        let f = from.0*8 + from.1; let t = to.0*8 + to.1;
        self.history[s][f][t]
    }
}


const MIN_EVAL_VALUE: i32 = i32::MIN + 100_000i32;
const MAX_EVAL_VALUE: i32 = i32::MAX - 100_000i32;

fn init_rayon_pool_if_needed() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // Prefer 12 threads by default; allow env override via RAYON_NUM_THREADS
        let default_threads = 12usize;
        let num_threads = std::env::var("RAYON_NUM_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(default_threads);
        // Increase worker thread stack size to avoid stack overflows in deep searches.
        let stack_bytes: usize = 32 * 1024 * 1024;
        let _ = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .stack_size(stack_bytes)
            .build_global();
    });
}

// --- Lightweight info callback support to report progress while searching ---
// Include UCI hashfull (permill) so GUI can display hash usage
type InfoCb = dyn Fn(((usize, usize), (usize, usize)), i32, usize, Vec<((usize, usize), (usize, usize))>, u16) + Send + Sync + 'static;
static INFO_CB: OnceLock<Mutex<Option<Arc<InfoCb>>>> = OnceLock::new();

fn info_cb_cell() -> &'static Mutex<Option<Arc<InfoCb>>> {
    INFO_CB.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_info_callback(cb: Option<Arc<InfoCb>>) {
    let cell = info_cb_cell();
    let mut guard = cell.lock().unwrap();
    *guard = cb;
}

fn emit_info(from: (usize, usize), to: (usize, usize), score_cp: i32, depth_used: usize, pv: Vec<((usize, usize), (usize, usize))>, hashfull_permille: u16) {
    if let Some(cb) = info_cb_cell().lock().unwrap().as_ref().cloned() {
        (cb)(((from.0, from.1), (to.0, to.1)), score_cp, depth_used, pv, hashfull_permille)
    }
}

// --- Global time budget (deadline) for search ---
static DEADLINE_CELL: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

#[inline]
fn deadline_cell() -> &'static Mutex<Option<Instant>> {
    DEADLINE_CELL.get_or_init(|| Mutex::new(None))
}

/// Set a hard time budget for the ongoing search. Passing 0 disables the budget.
pub(crate) fn set_time_budget_ms(ms: usize) {
    let mut guard = deadline_cell().lock().unwrap();
    if ms == 0 { *guard = None; } else { *guard = Some(Instant::now() + std::time::Duration::from_millis(ms as u64)); }
}

/// Clear any active time budget (search will run to completion by depth).
pub(crate) fn clear_time_budget() {
    let mut guard = deadline_cell().lock().unwrap();
    *guard = None;
}

#[inline]
fn time_is_up() -> bool {
    if let Some(dl) = *deadline_cell().lock().unwrap() {
        Instant::now() >= dl
    } else { false }
}

#[inline]
fn build_pv_for_root(
    board: &Board,
    root_side: Color,
    from: (usize, usize),
    to: (usize, usize),
    tt: &TranspositionTable,
    max_len: usize,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut pv: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(max_len.max(1));
    pv.push((from, to));

    // Work on a temporary board following the PV using TT best moves
    let mut tmp = board.clone();
    let _undo = make_move_simple(&mut tmp, from, to);
    let mut side = opposite_color(root_side);

    for _ in 1..max_len {
        let key = compute_zobrist(&tmp, side);
        let Some(entry) = tt.probe(key) else { break; };
        let (bf, bt) = (entry.best_from, entry.best_to);
        let ((nfr, nfc), (ntr, ntc)) = decode_move(bf, bt);
        let next = ((nfr, nfc), (ntr, ntc));
        // Validate legality in current position to avoid garbage PV
        let legals = find_all_valid_moves(&tmp, side);
        if !legals.contains(&next) { break; }
        pv.push(next);
        let _u = make_move_simple(&mut tmp, (nfr, nfc), (ntr, ntc));
        side = opposite_color(side);
    }
    pv
}

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
fn bump_node() {
    let _ = node_count_cell().fetch_add(1, Ordering::Relaxed);
}

/// Find the best move for the given game state, the search_depth, and the playing_strength
/// returns the evaluated score (in centipawns) for the selected move
/// and the effective search depth that was actually used internally.
pub(crate) fn find_move_with_info(
    game_state: GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), i32, usize)> {
    init_rayon_pool_if_needed();

    // Persistent Transposition Table across searches: initialize once and reuse.
    // We keep it behind a Mutex to allow mutable access in this serial root search.
    static TT_CELL: OnceLock<Mutex<TranspositionTable>> = OnceLock::new();
    let tt_mutex = TT_CELL.get_or_init(|| Mutex::new(TranspositionTable::new_with_default_size()));

    // collect all legal moves for the side to move
    let board = game_state.board();
    let active_color = game_state.active_color();
    let moves = find_all_valid_moves(board, active_color);

    if moves.is_empty() {
        return None;
    }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if search_depth == 0 { 1 } else { search_depth };

    // Map playing_strength [1..1000] to an effective depth to intentionally weaken play at low strengths.
    // Rough mapping: at ~300 strength, cap to ~3 ply; at 1000 keep requested depth.
    let ps = if playing_strength == 0 { 1 } else { playing_strength.min(1000) } as i32;

    /*let depth_min = 2i32; // never search less than 2 ply to avoid outright blunders like hanging queen immediately
    let depth_max = search_depth as i32;
    let effective_depth = if depth_max <= depth_min { depth_max } else {
        // linear interpolation between depth_min (weak) and depth_max (strong)
        let t = ps as f32 / 1000.0;
        let d = (depth_min as f32 + t * (depth_max as f32 - depth_min as f32)).round() as i32;
        d.clamp(depth_min, depth_max)
    } as usize;*/
    let effective_depth = search_depth;

    // Root-level hard 3-fold avoidance: filter out any root move that would create
    // a third occurrence of the same position (per truncated FEN used in History).
    // If filtering removes all moves (e.g., only repetition saves a loss), fall back to all moves.
    let root_moves: Vec<((usize, usize), (usize, usize))> = {
        let mut v = Vec::with_capacity(moves.len());
        for &(from, to) in &moves {
            let is_capture = board.get(to.0, to.1).is_some();
            let mut gs = game_state; // GameState is Copy
            let mut promote: Option<Piece> = None;
            if let Some(p) = gs.board().get(from.0, from.1) {
                if p.get_type() == PieceType::Pawn {
                    if (active_color == Color::White && to.0 == 7) || (active_color == Color::Black && to.0 == 0) {
                        promote = Some(Piece::new(PieceType::Queen, active_color));
                    }
                }
            }
            let makes_threefold = if PieceMover::move_piece(&mut gs, from, to, is_capture, promote) {
                gs.switch_player_turn();
                let fen = game_state_to_fen_string(gs);
                let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
                history.fen_repetition_count(&truncated) >= 2
            } else { false };
            if !makes_threefold { v.push((from, to)); }
        }
        if v.is_empty() { moves.clone() } else { v }
    };

    // Iterative Deepening + Aspiration windows at root (serial evaluation for stability)
    // Reuse persistent TT
    let mut tt = tt_mutex.lock().unwrap();
    let base_hmc = game_state.half_move_clock();
    let mut last_score: i32 = 0;
    let mut chosen: Option<((usize, usize), (usize, usize), i32, usize)> = None;
    let mut window: i32 = 50; // cp

    for depth_now in 1..=effective_depth {
        tt.next_age();
        let mut a = MIN_EVAL_VALUE + 1;
        let mut b = MAX_EVAL_VALUE - 1;
        if depth_now > 1 {
            a = (last_score - window).max(MIN_EVAL_VALUE + 1);
            b = (last_score + window).min(MAX_EVAL_VALUE - 1);
        }

        // Retry loop on aspiration fail
        let mut tried = 0;
        let best_tuple = loop {
            tried += 1;
            let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
            let mut best_score_raw = if active_color == Color::White { MIN_EVAL_VALUE } else { MAX_EVAL_VALUE };
            let mut best_adjusted = best_score_raw;

            // Order: if TT has a move at root, try to place it first
            let mut ordered = root_moves.clone();
            if let Some(entry) = tt.probe(compute_zobrist(&*board, active_color)) {
                let bm = decode_move(entry.best_from, entry.best_to);
                if let Some(pos) = ordered.iter().position(|m| *m == bm) {
                    let first = ordered.remove(pos);
                    ordered.insert(0, first);
                }
            }

            let enable_parallel = depth_now >= 6 && ordered.len() >= 4;
            if enable_parallel {
                // 1) Search the first (best-ordered) move serially to establish PV and bounds
                let &(pv_from, pv_to) = ordered.first().unwrap();
                {
                    let mut tmp = board.clone();
                    let u = make_move_simple(&mut tmp, pv_from, pv_to);
                    let moved_is_pawn = board.get(pv_from.0, pv_from.1).map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
                    let is_capture = board.get(pv_to.0, pv_to.1).is_some();
                    let child_hmc: u32 = if is_capture || moved_is_pawn { 0 } else { base_hmc.saturating_add(1) };
                    let score_raw = if depth_now <= 1 {
                        evaluate_position(&tmp)
                    } else {
                        let mut rep_stack: Vec<u64> = Vec::with_capacity(128);
                        alphabeta(&mut tmp, opposite_color(active_color), depth_now - 1, a, b, 1, &mut tt, child_hmc, &mut rep_stack)
                    };
                    unmake_move_simple(&mut tmp, u);

                    // Adjust score for root-only heuristics
                    let mut adjusted = score_raw + root_move_bonus(&board, pv_from, pv_to, active_color);
                    if let Some(captured) = board.get(pv_to.0, pv_to.1) {
                        use crate::piece::pieces::PieceType::*;
                        let cap_val = match captured.get_type() {
                            Pawn => 100, Knight => 320, Bishop => 330, Rook => 500, Queen => 900, King => 0,
                        };
                        adjusted += cap_val / 10;
                    }
                    let side_adv = if active_color == Color::White { score_raw } else { -score_raw };
                    if side_adv > 150 && base_hmc >= 80 {
                        if is_capture || moved_is_pawn {
                            let scale = (base_hmc as i32 - 79).min(21);
                            adjusted += 15 * scale;
                        } else {
                            let scale = (base_hmc as i32 - 79).min(21);
                            adjusted -= 8 * scale;
                        }
                    }
                    let sigma = strength_noise_sigma(ps as usize);
                    if sigma > 0 { let n: i32 = rng().random_range(-sigma..=sigma); adjusted += n; }

                    best_from_to = Some((pv_from, pv_to));
                    best_adjusted = adjusted;
                    best_score_raw = score_raw;
                }

                // 2) Search the remaining moves in parallel with per-task local TT to avoid contention
                let base_board = board.clone();
                let base_hmc_loc = base_hmc;
                let a_loc = a; let b_loc = b;
                let side = active_color;
                let results = ordered[1..].par_iter().map(|&(from, to)| {
                    let mut tmp = base_board.clone();
                    let u = make_move_simple(&mut tmp, from, to);
                    let moved_is_pawn = base_board.get(from.0, from.1).map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
                    let is_capture = base_board.get(to.0, to.1).is_some();
                    let child_hmc: u32 = if is_capture || moved_is_pawn { 0 } else { base_hmc_loc.saturating_add(1) };
                    let score_raw = if depth_now <= 1 {
                        evaluate_position(&tmp)
                    } else {
                        // local TT per task
                        let mut local_tt = TranspositionTable::new_with_default_size();
                        let mut rep_stack: Vec<u64> = Vec::with_capacity(128);
                        alphabeta(&mut tmp, opposite_color(side), depth_now - 1, a_loc, b_loc, 1, &mut local_tt, child_hmc, &mut rep_stack)
                    };
                    unmake_move_simple(&mut tmp, u);

                    // Root adjustments (skip repetition-history check to keep parallel code simple)
                    let mut adjusted = score_raw + root_move_bonus(&base_board, from, to, side);
                    if let Some(captured) = base_board.get(to.0, to.1) {
                        use crate::piece::pieces::PieceType::*;
                        let cap_val = match captured.get_type() {
                            Pawn => 100, Knight => 320, Bishop => 330, Rook => 500, Queen => 900, King => 0,
                        };
                        adjusted += cap_val / 10;
                    }
                    let side_adv = if side == Color::White { score_raw } else { -score_raw };
                    if side_adv > 150 && base_hmc_loc >= 80 {
                        if is_capture || moved_is_pawn {
                            let scale = (base_hmc_loc as i32 - 79).min(21);
                            adjusted += 15 * scale;
                        } else {
                            let scale = (base_hmc_loc as i32 - 79).min(21);
                            adjusted -= 8 * scale;
                        }
                    }
                    let sigma = strength_noise_sigma(ps as usize);
                    if sigma > 0 { let n: i32 = rng().random_range(-sigma..=sigma); adjusted += n; }
                    (from, to, adjusted, score_raw)
                }).reduce(|| {
                    // Identity: invalid move placeholder not used; return extreme sentinel
                    ((0usize,0usize), (0usize,0usize), if side==Color::White { MIN_EVAL_VALUE } else { MAX_EVAL_VALUE }, if side==Color::White { MIN_EVAL_VALUE } else { MAX_EVAL_VALUE })
                }, |acc, x| {
                    let better = if side == Color::White { x.2 > acc.2 } else { x.2 < acc.2 };
                    if better { x } else { acc }
                });

                // Update best with parallel results if better
                let (pf, pt, padj, praw) = results;
                // Ignore identity placeholder
                if !(pf == (0,0) && pt == (0,0)) {
                    let better = if active_color == Color::White { padj > best_adjusted } else { padj < best_adjusted };
                    if better {
                        best_from_to = Some((pf, pt));
                        best_adjusted = padj;
                        best_score_raw = praw;
                    }
                }
            } else {
                // Search sequentially over root moves
                for &(from, to) in &ordered {
                    let mut tmp = board.clone();
                    let u = make_move_simple(&mut tmp, from, to);
                    let moved_is_pawn = board.get(from.0, from.1).map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
                    let is_capture = board.get(to.0, to.1).is_some();
                    let child_hmc: u32 = if is_capture || moved_is_pawn { 0 } else { base_hmc.saturating_add(1) };
                let score_raw = if depth_now <= 1 {
                    evaluate_position(&tmp)
                } else {
                    let mut rep_stack: Vec<u64> = Vec::with_capacity(128);
                    alphabeta(&mut tmp, opposite_color(active_color), depth_now - 1, a, b, 1, &mut tt, child_hmc, &mut rep_stack)
                };
                unmake_move_simple(&mut tmp, u);

                // Adjust score for root-only heuristics
                let mut adjusted = score_raw + root_move_bonus(&board, from, to, active_color);
                if let Some(captured) = board.get(to.0, to.1) {
                    use crate::piece::pieces::PieceType::*;
                    let cap_val = match captured.get_type() {
                        Pawn => 100, Knight => 320, Bishop => 330, Rook => 500, Queen => 900, King => 0,
                    };
                    adjusted += cap_val / 10;
                }
                let side_adv = if active_color == Color::White { score_raw } else { -score_raw };
                if side_adv > 150 && base_hmc >= 80 {
                    if is_capture || moved_is_pawn {
                        let scale = (base_hmc as i32 - 79).min(21);
                        adjusted += 15 * scale;
                    } else {
                        let scale = (base_hmc as i32 - 79).min(21);
                        adjusted -= 8 * scale;
                    }
                }
                let sigma = strength_noise_sigma(ps as usize);
                if sigma > 0 { let n: i32 = rng().random_range(-sigma..=sigma); adjusted += n; }
                // repetition-avoidance at root
                {
                    let is_capture = board.get(to.0, to.1).is_some();
                    let mut gs = game_state; // Copy
                    let mut promote: Option<Piece> = None;
                    if let Some(p) = gs.board().get(from.0, from.1) {
                        if p.get_type() == PieceType::Pawn {
                            if (active_color == Color::White && to.0 == 7) || (active_color == Color::Black && to.0 == 0) {
                                promote = Some(Piece::new(PieceType::Queen, active_color));
                            }
                        }
                    }
                    if PieceMover::move_piece(&mut gs, from, to, is_capture, promote) {
                        gs.switch_player_turn();
                        let fen = game_state_to_fen_string(gs);
                        let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
                        let count = history.fen_repetition_count(&truncated);
                        let sa = if active_color == Color::White { adjusted } else { -adjusted };
                        if count >= 2 && sa > 0 {
                            adjusted -= if active_color == Color::White { 50_000 } else { -50_000 };
                        }
                    }
                }

                // Track best
                let better = if active_color == Color::White { adjusted > best_adjusted } else { adjusted < best_adjusted };
                if better || best_from_to.is_none() {
                    best_from_to = Some((from, to));
                    best_adjusted = adjusted;
                    best_score_raw = score_raw;
                }
                // Aspiration cutoffs help ordering mid-loop too
                    if active_color == Color::White && score_raw >= b { break; }
                    if active_color == Color::Black && score_raw <= a { break; }
                }
            }

            // Check aspiration result
            if best_score_raw <= a {
                // fail-low: widen down
                window = (window * 2).min(800);
                a = (last_score - window).max(MIN_EVAL_VALUE + 1);
                if tried < 3 { continue; }
            } else if best_score_raw >= b {
                // fail-high: widen up
                window = (window * 2).min(800);
                b = (last_score + window).min(MAX_EVAL_VALUE - 1);
                if tried < 3 { continue; }
            }
            break (best_from_to.unwrap(), best_adjusted, best_score_raw);
        };

        let ((bf, bt), best_adj, best_raw) = best_tuple;
        last_score = best_raw;
        // Emit PV/info for this iteration, including TT hashfull permille
        let pv = build_pv_for_root(board, active_color, bf, bt, &tt, depth_now);
        let hf = tt.hashfull_permille();
        emit_info(bf, bt, best_adj, depth_now, pv, hf);
        chosen = Some((bf, bt, best_adj, depth_now));
    }

    // Final selection based on playing_strength from the last iteration
    if let Some((bf, bt, sc, used_depth)) = chosen {
        if playing_strength >= 1000 {
            Some((bf, bt, sc, used_depth))
        } else {
            // Re-evaluate top K moves for stochastic selection at final depth
            let mut scored: Vec<((usize, usize), (usize, usize), i32)> = Vec::new();
            for &(from, to) in &root_moves {
                // Use TT best estimates (no re-search) for quick ranking
                let mut tmp = board.clone();
                let u = make_move_simple(&mut tmp, from, to);
                let moved_is_pawn = board.get(from.0, from.1).map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
                let is_capture = board.get(to.0, to.1).is_some();
                let child_hmc: u32 = if is_capture || moved_is_pawn { 0 } else { base_hmc.saturating_add(1) };
                let mut rep_stack: Vec<u64> = Vec::with_capacity(64);
                let sr = if effective_depth <= 1 { evaluate_position(&tmp) } else { alphabeta(&mut tmp, opposite_color(active_color), effective_depth - 1, MIN_EVAL_VALUE+1, MAX_EVAL_VALUE-1, 1, &mut tt, child_hmc, &mut rep_stack) };
                unmake_move_simple(&mut tmp, u);
                let adj = sr + root_move_bonus(&board, from, to, active_color);
                scored.push((from, to, adj));
            }
            let mut sorted = sort_moves_on_score_asc(&mut scored);
            if active_color == Color::White { sorted.reverse(); }
            if let Some((from, to)) = select_move_based_using_strength(&sorted, playing_strength) {
                let sc = sorted.iter().find(|e| e.0==from && e.1==to).map(|e| e.2).unwrap_or(sc);
                Some((from, to, sc, used_depth))
            } else { Some((bf, bt, sc, used_depth)) }
        }
    } else { None }
}

pub(crate) fn find_all_valid_moves(board: &Board, active_color:Color) -> Vec<((usize, usize), (usize, usize))> {
    let mut result: Vec<((usize, usize), (usize, usize))> = Vec::new();

    // iterate all squares and collect legal moves for the active color
    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) { Some(p) => p, None => continue };
            if piece.get_color() != active_color { continue; }

            for tr in 0..8 {
                for tc in 0..8 {
                    let from = (r, c);
                    let to = (tr, tc);
                    if from == to { continue; }

                    let target_piece_is_some = board.get(tr, tc).is_some();

                    // basic board-level validation (ownership, capture flags, bounds)
                    let is_capture = target_piece_is_some;
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !board.move_from_and_to_validation_check(from, to, active_color, is_capture, is_pawn_move, None) {
                        continue;
                    }

                    if is_piece_move_valid(board, active_color, r, c, piece, tr, tc, from, to, is_capture) {
                        result.push((from, to));
                    }
                }
            }
        }
    }
    result
}

// Small, root-level heuristic bonus used to break ties at low depth.
// Positive favors White; negative favors Black (we add for side to move).
fn root_move_bonus(board: &Board, from: (usize, usize), to: (usize, usize), side: Color) -> i32 {
    let mut bonus: i32 = 0;

    // Identify piece and basic metadata
    let piece = match board.get(from.0, from.1) { Some(p) => p, None => return 0 };
    let pt = piece.get_type();

    // Opening-principle nudges (very small):
    // - prefer central pawn advances (d/e pawns); discourage a/h pawn pushes
    // - prefer knights to c3/f3 and bishops to c4/f4 for White (mirror for Black)
    let (fr, fc) = from;
    let (tr, tc) = to;

    // discourage rook pawns (files a/h -> col 0/7) pushing as early plan
    if pt == PieceType::Pawn && (fc == 0 || fc == 7) {
        // stronger if double push (two ranks)
        let dr = if fr > tr { fr as i32 - tr as i32 } else { tr as i32 - fr as i32 };
        bonus -= if dr >= 2 { 35 } else { 25 };
    }

    // prefer central pawn advances on d/e files, especially 2-step from home
    if pt == PieceType::Pawn && (fc == 3 || fc == 4) {
        let dr = if fr > tr { fr as i32 - tr as i32 } else { tr as i32 - fr as i32 };
        bonus += if dr >= 2 { 35 } else { 20 };
    }

    // Knights to c3/f3 (White) or c6/f6 (Black)
    if pt == PieceType::Knight {
        match side {
            Color::White => {
                if (tr, tc) == (2, 2) || (tr, tc) == (2, 5) { bonus += 20; }
            }
            Color::Black => {
                if (tr, tc) == (5, 2) || (tr, tc) == (5, 5) { bonus += 20; }
            }
        }
    }

    // Bishops to c4/f4 for White; c5/f5 for Black
    if pt == PieceType::Bishop {
        match side {
            Color::White => { if (tr, tc) == (3, 2) || (tr, tc) == (3, 5) { bonus += 12; } }
            Color::Black => { if (tr, tc) == (4, 2) || (tr, tc) == (4, 5) { bonus += 12; } }
        }
    }

    // Very small central control nudge for landing on or influencing center rings
    let central_files = tc >= 2 && tc <= 5; // c..f
    let central_ranks_white = tr >= 2 && tr <= 4; // ranks 3..5 from White pov
    let central_ranks_black = tr >= 3 && tr <= 5; // ranks 4..6 from White rows ~ Black push
    if central_files && ((side == Color::White && central_ranks_white) || (side == Color::Black && central_ranks_black)) {
        bonus += 5;
    }

    // Apply sign for side to move (we always add for the maximizing side at root)
    match side {
        Color::White => bonus,
        Color::Black => -bonus,
    }
}

fn is_piece_move_valid(board: &Board, active_color: Color, r: usize, c: usize, piece: Piece, tr: usize, tc: usize, from: (usize, usize), to: (usize, usize), is_capture: bool) -> bool {
    // piece-type specific path/shape validation including pin checks
    let mut tmp = board.clone();
    let ok = match piece.get_type() {
        PieceType::Pawn => is_valid_pawn_move(&mut tmp, from, to, is_capture, None, active_color, None, true),
        PieceType::Knight => is_valid_knight_move(&mut tmp, from, to, true),
        PieceType::Bishop => is_valid_bishop_move(&mut tmp, from, to, true),
        PieceType::Rook => is_valid_rook_move(&mut tmp, from, to, true),
        PieceType::Queen => is_valid_queen_move(&mut tmp, from, to, true),
        PieceType::King => {
            // king: allow single-square moves that do not move into check (no castling here)
            let dr = if r > tr { r - tr } else { tr - r };
            let dc = if c > tc { c - tc } else { tc - c };
            if dr <= 1 && dc <= 1 {
                // ensure the king wouldn't be in check after the move
                !is_king_in_check_after_move(&mut tmp, from, to, None)
            } else { false }
        }
    };
    ok
}

#[inline]
fn opposite_color(c: Color) -> Color {
    match c { Color::White => Color::Black, Color::Black => Color::White }
}

// --- Lightweight helpers for selective extensions ---
// Compute a simple material-based game phase similar to evaluator (0..24)
#[inline]
fn game_phase_light(board: &Board) -> i32 {
    const PHASE_KNIGHT: i32 = 1;
    const PHASE_BISHOP: i32 = 1;
    const PHASE_ROOK: i32 = 2;
    const PHASE_QUEEN: i32 = 4;
    let mut phase: i32 = 0;
    for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
        phase += match p.get_type() {
            PieceType::Knight => PHASE_KNIGHT,
            PieceType::Bishop => PHASE_BISHOP,
            PieceType::Rook => PHASE_ROOK,
            PieceType::Queen => PHASE_QUEEN,
            _ => 0,
        };
    }}}
    if phase < 0 { 0 } else if phase > 24 { 24 } else { phase }
}

// Determine if a pawn at (row,col) for color is a passed pawn (no enemy pawns ahead on same/adjacent files)
#[inline]
fn is_passed_pawn_simple(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let dir: i32 = if color == Color::White { 1 } else { -1 };
    let mut r = row as i32 + dir;
    while r >= 0 && r < 8 {
        for dc in [-1i32, 0, 1] {
            let nc = col as i32 + dc;
            if nc < 0 || nc >= 8 { continue; }
            if let Some(p) = board.get(r as usize, nc as usize) {
                if p.get_color() != color && p.get_type() == PieceType::Pawn { return false; }
            }
        }
        r += dir;
    }
    true
}

// Alpha-beta pruning search. Returns evaluation in centipawns (positive is better for White).
fn alphabeta(
    board: &mut Board,
    to_move: Color,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    halfmove_clock: u32,
    rep_stack: &mut Vec<u64>,
) -> i32 {
    // Count every node we enter
    bump_node();

    // Time cutoff: on timeout, return a static evaluation of the current node.
    // This ensures callers can still use best-so-far information gathered so far.
    if time_is_up() {
        return evaluate_position(&*board);
    }

    // print board
    //println!("alpha-beta\n{}", board.get_board_display_string(None));

    // Repetition/50-move draw checks at node entry
    let key_here = compute_zobrist(&*board, to_move);
    // If this key already exists in the current line, it's a repetition -> draw
    if rep_stack.iter().any(|&k| k == key_here) {
        return 0;
    }
    // 50-move rule
    if halfmove_clock >= 100 {
        return 0;
    }

    if depth == 0 {
        // At leaf: switch to quiescence to avoid horizon effects
        return qsearch(board, to_move, alpha, beta, halfmove_clock, rep_stack);
    }

    // -----------------
    // Null-move pruning
    // -----------------
    // Conditions to attempt null move:
    // - Sufficient remaining depth
    // - Side to move is not in check
    // - Halfmove clock not already at draw threshold
    // - Avoid in likely zugzwang scenarios (very low material) — here we approximate by requiring some non-pawn material
    if depth >= 3 {
        let in_check = is_side_in_check(board, to_move);
        if !in_check && halfmove_clock < 100 {
            // Quick material heuristic: require presence of any piece other than kings/pawns
            let mut has_non_pawn_minor = false;
            'scan: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() == to_move {
                            match p.get_type() {
                                PieceType::Knight | PieceType::Bishop | PieceType::Rook | PieceType::Queen => {
                                    has_non_pawn_minor = true; break 'scan;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if has_non_pawn_minor {
                // Reduction R: base on depth (deeper search -> larger R)
                let r = if depth >= 6 { 3 } else { 2 } as usize;
                let undo: Option<()> = None;
                // Make a null move: switch side to move without changing board, but we still push repetition key
                // We reuse halfmove_clock (null move does not reset it)
                // To keep Book/Board API simple, emulate by switching to_move only in recursive call
                // and NOT modifying the board.
                // Probe a null-window search; use (beta-1, beta) window which is standard for NMP
                let score = alphabeta(board, opposite_color(to_move), depth.saturating_sub(1 + r), beta - 1, beta, ply + 1, tt, halfmove_clock.saturating_add(1), rep_stack);
                let _ = undo; // placeholder for symmetry with real moves; no board change was made
                if score >= beta {
                    return score; // null-move cutoff
                }
            }
        }
    }

    // TT probe
    let key = key_here;
    if let Some(entry) = tt.probe(key) {
        if entry.depth as usize >= depth {
            let tt_score = from_tt_score(entry.score, ply);
            match entry.bound {
                Bound::Exact => { return tt_score; }
                Bound::Lower => {
                    if tt_score >= beta { return tt_score; }
                    if tt_score > alpha { alpha = tt_score; }
                }
                Bound::Upper => {
                    if tt_score <= alpha { return tt_score; }
                    if tt_score < beta { beta = tt_score; }
                }
            }
        }
    }

    let mut moves = find_all_valid_moves(&*board, to_move);
    // If TT has a best move, try it first
    if let Some(entry) = tt.probe(key) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = moves.iter().position(|m| *m == bm) {
            let first = moves.remove(pos);
            moves.insert(0, first);
        }
    }
    // Basic move ordering with heuristics: after TT move, sort by composite key
    if moves.len() > 1 {
        // Stable-partition: keep index 0 (possibly TT move) in place, sort the tail
        let (head, tail) = moves.split_at_mut(1);
        let board_ref = &*board;
        let hmc = halfmove_clock;
        // Access history/killer tables
        thread_local! {
            static HEUR: OnceLock<Mutex<SearchHeuristics>> = OnceLock::new();
        }
        tail.sort_by_key(|&(from, to)| {
            // Base MVV-LVA score
            let mut key = move_score_mvv_lva(board_ref, from, to);
            let moved_is_pawn = board_ref.get(from.0, from.1).map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
            let is_capture = board_ref.get(to.0, to.1).is_some();
            // DTZ-like ordering bump: near 50-move horizon, prioritize pawn moves and captures
            if hmc >= 80 {
                if moved_is_pawn || is_capture { key += 100_000; }
            }
            // Killer move bonus (only for quiets)
            if !is_capture && !moved_is_pawn {
                let is_killer = HEUR.with(|h| {
                    let m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                    m.is_killer(ply as usize, from, to)
                });
                if is_killer { key += 200_000; }
                // History bonus
                let hist = HEUR.with(|h| {
                    let m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                    m.history_score(to_move, from, to)
                });
                // Scale down history to be commensurate with MVV-LVA units
                key += (hist / 32).clamp(-200_000, 200_000);
            }
            -key
        });
        // head is unused, only here to make split_at_mut compile
        let _ = head;
    }
    if moves.is_empty() {
        // No legal moves: checkmate or stalemate
        let in_check = is_side_in_check(board, to_move);
        if in_check {
            // Losing side to move is checkmated. Use large negative for side to move.
            // Depth-based bonus (the sooner the mate, the larger the magnitude):
            // With our interface lacking ply, approximate using remaining depth.
            return -MATE_VALUE + depth as i32;
        } else {
            // stalemate: draw
            return 0;
        }
    }

    let original_alpha = alpha;
    let original_beta = beta;
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    // Push current position to repetition stack for descendants
    rep_stack.push(key_here);

    // Create/extend search heuristics container (stack-allocated per call chain depth)
    // We pass it implicitly via thread-local static since function signatures are fixed; for simplicity,
    // keep a single heuristics instance at root using OnceLock. This is conservative but effective.
    thread_local! {
        static HEUR: OnceLock<Mutex<SearchHeuristics>> = OnceLock::new();
    }
    let value_holder;
    let value = if to_move == Color::White {
        let mut value = MIN_EVAL_VALUE;
        let mut is_first_move = true; // PVS: first move searched with full window
        let mut move_index: i32 = 0;  // LMR: track move order
        for (from, to) in moves.into_iter() {
            // Detect moved piece before making the move
            let moved_piece = board.get(from.0, from.1);
            let target_piece = board.get(to.0, to.1);
            let u = make_move_simple(board, from, to);
            // Passed-pawn push extension (B5): If a pawn move results in a passed pawn
            // reaching the 6th/7th rank (relative to the side) in a near-endgame, extend by +1 ply.
            let mut child_depth = depth.saturating_sub(1);
            // Track halfmove clock: reset on pawn move or capture
            let mut child_hmc = halfmove_clock + 1;
            if let Some(p) = moved_piece {
                if p.get_type() == PieceType::Pawn {
                    child_hmc = 0;
                    let color = p.get_color();
                    let r = to.0; let c = to.1;
                    if game_phase_light(board) <= 8 && is_passed_pawn_simple(board, r, c, color) {
                        let adv: i32 = match color { Color::White => r as i32, Color::Black => (7 - r) as i32 };
                        if adv >= 5 { // 6th or 7th rank
                            child_depth = child_depth.saturating_add(1);
                        }
                    }
                }
            }
            if target_piece.is_some() { child_hmc = 0; }

            // History/Killer move ordering boost for quiet moves (after initial TT/MVV ordering)
            let is_capture = target_piece.is_some();
            let is_pawn_move = moved_piece.map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
            let quiet = !is_capture && !is_pawn_move;

            // Principal Variation Search (PVS) + Late Move Reductions (LMR)
            let mut score;
            if is_first_move {
                // Full window on the first move
                score = alphabeta(board, Color::Black, child_depth, alpha, beta, ply + 1, tt, child_hmc, rep_stack);
                is_first_move = false;
            } else {
                // Late Move Reduction
                let mut reduced_depth = child_depth;
                if quiet && child_depth >= 3 && move_index >= 3 {
                    // Basic reduction formula: grows with move index and depth
                    // Use history to avoid over-reducing historically good moves
                    let hist = HEUR.with(|h| {
                        let m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                        m.history_score(to_move, from, to)
                    });
                    let hist_good = hist > 10_000; // tuned threshold
                    let r = 1 + ((move_index as usize) / 6).min(2) + ((child_depth as usize) / 6).min(1) - if hist_good { 1 } else { 0 };
                    reduced_depth = reduced_depth.saturating_sub(r);
                }
                // Null-window search for subsequent moves (PVS window)
                score = alphabeta(board, Color::Black, reduced_depth, alpha, alpha + 1, ply + 1, tt, child_hmc, rep_stack);
                if score > alpha && score < beta {
                    // Re-search with full window on fail-high/improvement inside window
                    score = alphabeta(board, Color::Black, child_depth, alpha, beta, ply + 1, tt, child_hmc, rep_stack);
                }
            }
            unmake_move_simple(board, u);
            if score > value { value = score; }
            if value > alpha { 
                alpha = value; best_from_to = Some((from, to));
                // On alpha improvement, update history for quiets
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score >= beta {
                // Beta cutoffs are handled by loop break, but record killer before break
                HEUR.with(|h| {
                    let mut m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
            if alpha >= beta { break; }
            move_index += 1;
        }
        value_holder = value; value_holder
    } else {
        let mut value = MAX_EVAL_VALUE;
        let mut is_first_move = true; // PVS for minimizing side too
        let mut move_index: i32 = 0;  // LMR index
        for (from, to) in moves.into_iter() {
            // Detect moved piece before making the move
            let moved_piece = board.get(from.0, from.1);
            let target_piece = board.get(to.0, to.1);
            let u = make_move_simple(board, from, to);
            // Passed-pawn push extension (B5)
            let mut child_depth = depth.saturating_sub(1);
            // Track halfmove clock for child
            let mut child_hmc = halfmove_clock + 1;
            if let Some(p) = moved_piece {
                if p.get_type() == PieceType::Pawn {
                    child_hmc = 0;
                    let color = p.get_color();
                    let r = to.0; let c = to.1;
                    if game_phase_light(board) <= 8 && is_passed_pawn_simple(board, r, c, color) {
                        let adv: i32 = match color { Color::White => r as i32, Color::Black => (7 - r) as i32 };
                        if adv >= 5 {
                            child_depth = child_depth.saturating_add(1);
                        }
                    }
                }
            }
            if target_piece.is_some() { child_hmc = 0; }
            // PVS + LMR for minimizing side
            let is_capture = target_piece.is_some();
            let is_pawn_move = moved_piece.map(|p| p.get_type()==PieceType::Pawn).unwrap_or(false);
            let quiet = !is_capture && !is_pawn_move;
            let mut score;
            if is_first_move {
                score = alphabeta(board, Color::White, child_depth, alpha, beta, ply + 1, tt, child_hmc, rep_stack);
                is_first_move = false;
            } else {
                // Late Move Reduction
                let mut reduced_depth = child_depth;
                if quiet && child_depth >= 3 && move_index >= 3 {
                    let hist = HEUR.with(|h| {
                        let m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                        m.history_score(to_move, from, to)
                    });
                    let hist_good = hist < -10_000; // minimizing side: negative is good for minimizing? keep symmetric for simplicity
                    let r = 1 + ((move_index as usize) / 6).min(2) + ((child_depth as usize) / 6).min(1) - if hist_good { 1 } else { 0 };
                    reduced_depth = reduced_depth.saturating_sub(r);
                }
                // For the minimizing side, a null-window around beta-1 .. beta works equivalently
                score = alphabeta(board, Color::White, reduced_depth, beta - 1, beta, ply + 1, tt, child_hmc, rep_stack);
                if score < beta && score > alpha {
                    score = alphabeta(board, Color::White, child_depth, alpha, beta, ply + 1, tt, child_hmc, rep_stack);
                }
            }
            unmake_move_simple(board, u);
            if score < value { value = score; }
            if value < beta { 
                beta = value; best_from_to = Some((from, to));
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score <= alpha {
                HEUR.with(|h| {
                    let mut m = h.get_or_init(|| Mutex::new(SearchHeuristics::new(128))).lock().unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
            if alpha >= beta { break; }
            move_index += 1;
        }
        value_holder = value; value_holder
    };

    // Pop this node key
    let _ = rep_stack.pop();

    // Store to TT
    let bound = if value <= original_alpha { Bound::Upper }
                else if value >= original_beta { Bound::Lower }
                else { Bound::Exact };
    let (bf, bt) = if let Some((f, t)) = best_from_to { let (ff, tt2) = encode_move(f, t); (Some(ff), Some(tt2)) } else { (None, None) };
    let tt_score = to_tt_score(value, ply);
    tt.store(key, depth as i16, bound, tt_score, bf, bt);
    value
}

// Quiescence search: consider only tactical continuations (captures) unless in check.
fn qsearch(board: &mut Board, to_move: Color, mut alpha: i32, beta: i32, halfmove_clock: u32, rep_stack: &mut Vec<u64>) -> i32 {
    // Time cutoff in quiescence as well: return a quick static eval
    if time_is_up() {
        return evaluate_position(&*board);
    }
    // Draw checks in quiescence as well
    let key_here = compute_zobrist(&*board, to_move);
    if rep_stack.iter().any(|&k| k == key_here) { return 0; }
    if halfmove_clock >= 100 { return 0; }

    // Stand-pat (static) evaluation. Suppress stand-pat only when in check.
    let in_check = is_side_in_check(board, to_move);
    let stand_pat = evaluate_position(&*board);
    if !in_check {
        // Uniform alpha/beta semantics regardless of side-to-move.
        if stand_pat >= beta { return stand_pat; }
        if stand_pat > alpha { alpha = stand_pat; }
    }

    // Generate moves. If not in check, restrict to captures (quiescence).
    // NOTE: For speed we currently generate all and filter; consider adding
    // a dedicated capture generator to avoid the extra work.
    let mut moves = find_all_valid_moves(&*board, to_move);
    if !in_check {
        moves.retain(|&(_f, to)| board.get(to.0, to.1).is_some());
    }

    // Selective endgame pawn-push quiescence: allow a few safe passer pushes
    // to stabilize eval around promotion races. Tight gating to avoid explosion.
    if !in_check {
        let phase = game_phase_light(&*board);
        if phase <= 8 {
            // collect up to N safe quiet pushes
            const MAX_QUIET_PUSHES: usize = 2;
            let mut added: usize = 0;
            'outer: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() != to_move || p.get_type() != PieceType::Pawn { continue; }
                        // only consider passed pawns on 5th–7th ranks (relative to side)
                        let adv: i32 = match to_move { Color::White => r as i32, Color::Black => (7 - r) as i32 };
                        if adv < 4 { continue; }
                        if !is_passed_pawn_simple(&*board, r, c, to_move) { continue; }
                        // one-step push target
                        let (nr_opt, to_sq) = match to_move {
                            Color::White => (if r < 7 { Some(r + 1) } else { None }, (r.saturating_add(1), c)),
                            Color::Black => (if r > 0 { Some(r - 1) } else { None }, (r.saturating_sub(1), c)),
                        };
                        let nr = if let Some(nr) = nr_opt { nr } else { continue; };
                        if board.get(nr, c).is_some() { continue; }
                        // simulate and verify safety and legality
                        let from = (r, c);
                        let to = to_sq;
                        let u = make_move_simple(board, from, to);
                        // move must not leave own king in check
                        let illegal = is_side_in_check(board, to_move);
                        // target square should not be immediately attacked by opponent
                        use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                        let attacked = is_square_attacked_by_opponent(board, to, to_move);
                        unmake_move_simple(board, u);
                        if illegal || attacked { continue; }
                        moves.push((from, to));
                        added += 1;
                        if added >= MAX_QUIET_PUSHES { break 'outer; }
                    }
                }
            }
        }
    }

    // Delta pruning: if not in check and clearly below alpha, prune.
    // Start with a conservative constant margin; tune empirically.
    const DELTA_MARGIN: i32 = 150; // centipawns
    if !in_check && stand_pat + DELTA_MARGIN <= alpha {
        return stand_pat;
    }

    if moves.is_empty() {
        return stand_pat;
    }

    // Order captures by MVV-LVA to improve cutoffs (only matters for captures branch)
    if !in_check {
        let b = &*board;
        moves.sort_by_key(|&(from, to)| -move_score_mvv_lva(&b, from, to));
    }

    // Simple capture SEE-like filter: skip obviously losing captures when not in check.
    // Uses basic piece values; this is a cheap approximation, not true SEE.
    #[inline]
    fn piece_simple_value(p: PieceType) -> i32 {
        use crate::piece::pieces::PieceType::*;
        match p { Pawn=>100, Knight=>320, Bishop=>330, Rook=>500, Queen=>900, King=>20000 }
    }

    let mut a = alpha;
    let mut bnd = beta;
    if to_move == Color::White {
        let mut best = MIN_EVAL_VALUE;
        for (from, to) in moves.into_iter() {
            if !in_check {
                if let (Some(att), Some(vic)) = (board.get(from.0, from.1), board.get(to.0, to.1)) {
                    let att_v = piece_simple_value(att.get_type());
                    let vic_v = piece_simple_value(vic.get_type());
                    // Skip "bad" captures where attacker is significantly more valuable than victim
                    if vic_v + 50 < att_v { continue; }
                    // Futility in qsearch (White to move): if even taking the victim cannot raise alpha, skip
                    const FUT_MARGIN: i32 = 50;
                    if stand_pat + vic_v + FUT_MARGIN <= a { continue; }
                }
            }
            let was_capture = board.get(to.0, to.1).is_some();
            let u = make_move_simple(board, from, to);
            let mut child_hmc = halfmove_clock + 1;
            if was_capture { child_hmc = 0; }
            let score = qsearch(board, Color::Black, a, bnd, child_hmc, rep_stack);
            unmake_move_simple(board, u);
            if score > best { best = score; }
            if best > a { a = best; }
            if a >= bnd { break; }
        }
        best
    } else {
        let mut best = MAX_EVAL_VALUE;
        for (from, to) in moves.into_iter() {
            if !in_check {
                if let (Some(att), Some(vic)) = (board.get(from.0, from.1), board.get(to.0, to.1)) {
                    let att_v = piece_simple_value(att.get_type());
                    let vic_v = piece_simple_value(vic.get_type());
                    if vic_v + 50 < att_v { continue; }
                    // Futility in qsearch (Black to move): if even taking the victim cannot drop below beta, skip
                    const FUT_MARGIN: i32 = 50;
                    if stand_pat - vic_v - FUT_MARGIN >= bnd { continue; }
                }
            }
            let was_capture = board.get(to.0, to.1).is_some();
            let u = make_move_simple(board, from, to);
            let mut child_hmc = halfmove_clock + 1;
            if was_capture { child_hmc = 0; }
            let score = qsearch(board, Color::White, a, bnd, child_hmc, rep_stack);
            unmake_move_simple(board, u);
            if score < best { best = score; }
            if best < bnd { bnd = best; }
            if a >= bnd { break; }
        }
        best
    }
}

// Heuristic score for move ordering: MVV-LVA (Most Valuable Victim - Least Valuable Attacker)
#[inline]
fn move_score_mvv_lva(board: &Board, from: (usize, usize), to: (usize, usize)) -> i32 {
    use crate::piece::pieces::PieceType::*;
    let victim = board.get(to.0, to.1);
    let attacker = board.get(from.0, from.1);
    let v = victim.map(|p| match p.get_type() { Pawn=>100, Knight=>320, Bishop=>330, Rook=>500, Queen=>900, King=>20000 }).unwrap_or(0);
    let a = attacker.map(|p| match p.get_type() { Pawn=>100, Knight=>320, Bishop=>330, Rook=>500, Queen=>900, King=>20000 }).unwrap_or(0);
    // Higher is better for ordering. Captures first; quiets get 0 or negative.
    if victim.is_some() { v * 100 - a } else { -1 }
}

/// Helper: is the given side to move currently in check on this board state?
fn is_side_in_check(board: &mut Board, side: Color) -> bool {
    use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
    let king_sq = board.get_king_location(side);
    is_square_attacked_by_opponent(board, king_sq, side)
}

// Lightweight, reversible move helpers for search. These intentionally do not
// handle special moves (castling, en-passant, promotions) because our search
// move generator and validators do not invoke them through this path either.
// They mirror the simple behavior of Board::move_piece/move_pawn above.
#[derive(Clone, Copy)]
struct UndoMove {
    from: (usize, usize),
    to: (usize, usize),
    moved: Option<Piece>,
    captured: Option<Piece>,
    // Save king locations to restore accurately on unmake
    prev_white_king: (usize, usize),
    prev_black_king: (usize, usize),
}

#[inline]
fn make_move_simple(board: &mut Board, from: (usize, usize), to: (usize, usize)) -> UndoMove {
    let moved = board.get(from.0, from.1);
    let captured = board.get(to.0, to.1);
    // snapshot king locations before move
    let prev_white_king = board.get_king_location(Color::White);
    let prev_black_king = board.get_king_location(Color::Black);
    // apply move directly
    board.set(to.0, to.1, moved);
    board.set(from.0, from.1, None);
    // update king location cache if a king moved
    if let Some(p) = moved {
        if p.get_type() == PieceType::King {
            board.set_king_location(p.get_color(), to);
        }
    }
    UndoMove { from, to, moved, captured, prev_white_king, prev_black_king }
}

#[inline]
fn unmake_move_simple(board: &mut Board, undo: UndoMove) {
    // restore original squares
    board.set(undo.from.0, undo.from.1, undo.moved);
    board.set(undo.to.0, undo.to.1, undo.captured);
    // restore king location cache from snapshot
    board.set_king_location(Color::White, undo.prev_white_king);
    board.set_king_location(Color::Black, undo.prev_black_king);
}

// Sorts the move table by score in ascending order and returns a cloned, sorted vector.
fn sort_moves_on_score_asc(
    move_table: &mut Vec<((usize, usize), (usize, usize), i32)>
) -> Vec<((usize, usize), (usize, usize), i32)> {
    move_table.sort_by_key(|m| m.2);
    move_table.clone()
}

// Controlled by the strength parameter, the search will not always return the best move.
// Selects randomly among the best-scoring moves in a sorted (ascending) move table.
fn select_move_based_using_strength(
    sorted_moves: &Vec<((usize, usize), (usize, usize), i32)>, playing_strength: usize
) -> Option<((usize, usize), (usize, usize))> {

    if sorted_moves.is_empty() { return None; }

    // Clamp strength to [1..1000]
    let ps = if playing_strength == 0 { 1 } else { playing_strength.min(1000) };

    // Blunder chance: with some probability (higher when weaker), deliberately pick from the bottom of list.
    // This creates human-like mistakes at low skill.
    let blunder_chance = if ps >= 950 { 0.0 }
        else if ps >= 800 { 0.01 }
        else if ps >= 650 { 0.03 }
        else if ps >= 500 { 0.05 }
        else if ps >= 350 { 0.10 }
        else { 0.18 };
    let roll: f32 = rng().random::<f32>();
    if roll < blunder_chance {
        // pick from bottom bucket (worst moves), limited to 30% of list but at least 2 moves
        let len = sorted_moves.len();
        let bucket = (len as f32 * 0.30).ceil() as usize;
        let bucket = bucket.max(2).min(len);
        let start = len - bucket;
        let idx = rng().random_range(start..len);
        let pick = &sorted_moves[idx];
        return Some((pick.0, pick.1));
    }

    // Choose from top-K based on strength. For low strength pick from a wider bucket,
    // but still bias the pick toward the best move within that bucket.
    // Map strength to K in [len, 1] roughly: strong -> pick among top 1..3, weak -> wider.
    let len = sorted_moves.len();
    // Limit randomness to top 6 to avoid clearly dubious opening moves surfacing too often.
    let max_bucket = len.min(6);
    let k = if ps >= 950 { 1 }
            else if ps >= 800 { 2 }
            else if ps >= 650 { 3 }
            else if ps >= 500 { 4 }
            else if ps >= 350 { 5 }
            else if ps >= 200 { 6 }
            else { 8 };
    let k = k.min(max_bucket).max(1);

    // Random index within top-k, biased toward 0 (best move).
    // Use the minimum of two uniform draws to skew toward lower indices.
    let r1: usize = rng().random_range(0..k);
    let r2: usize = rng().random_range(0..k);
    let idx = r1.min(r2);
    let pick = &sorted_moves[idx];
    Some((pick.0, pick.1))
}

// Map strength to evaluation noise (centipawns). 0 at 1000, higher at low strengths.
#[inline]
fn strength_noise_sigma(ps: usize) -> i32 {
    let ps = ps.min(1000).max(1) as i32;
    // Piecewise linear: ~200cp at ps=1, ~120cp at ps=300, ~0 at 1000
    let sigma = if ps >= 1000 { 0 }
        else if ps >= 700 { ((1000 - ps) as f32 * 0.10) as i32 }  // up to ~30cp
        else if ps >= 400 { ((700 - ps) as f32 * 0.20 + 30.0) as i32 } // ~30..90
        else { ((400 - ps) as f32 * 0.30 + 90.0) as i32 }; // up to ~210
    sigma.max(0)
}
