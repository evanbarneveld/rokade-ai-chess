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
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};


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
type InfoCb = dyn Fn(((usize, usize), (usize, usize)), i32, usize, Vec<((usize, usize), (usize, usize))>) + Send + Sync + 'static;
static INFO_CB: OnceLock<Mutex<Option<Arc<InfoCb>>>> = OnceLock::new();

fn info_cb_cell() -> &'static Mutex<Option<Arc<InfoCb>>> {
    INFO_CB.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_info_callback(cb: Option<Arc<InfoCb>>) {
    let cell = info_cb_cell();
    let mut guard = cell.lock().unwrap();
    *guard = cb;
}

fn emit_info(from: (usize, usize), to: (usize, usize), score_cp: i32, depth_used: usize, pv: Vec<((usize, usize), (usize, usize))>) {
    if let Some(cb) = info_cb_cell().lock().unwrap().as_ref().cloned() {
        (cb)(((from.0, from.1), (to.0, to.1)), score_cp, depth_used, pv)
    }
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
///
pub (crate) fn find_move(game_state: GameState, history: &History, search_depth: usize, playing_strength:usize) -> Option<((usize, usize), (usize, usize))> {
    // Keep existing API by delegating to the info-enabled variant
    match find_move_with_info(game_state, history, search_depth, playing_strength) {
        Some((from, to, _score_cp, _depth_used)) => Some((from, to)),
        None => None,
    }
}

/// Like `find_move` but also returns the evaluated score (in centipawns) for the selected move
/// and the effective search depth that was actually used internally.
pub(crate) fn find_move_with_info(
    game_state: GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), i32, usize)> {
    init_rayon_pool_if_needed();

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
    let depth_min = 2i32; // never search less than 2 ply to avoid outright blunders like hanging queen immediately
    let depth_max = search_depth as i32;
    let effective_depth = if depth_max <= depth_min { depth_max } else {
        // linear interpolation between depth_min (weak) and depth_max (strong)
        let t = ps as f32 / 1000.0;
        let d = (depth_min as f32 + t * (depth_max as f32 - depth_min as f32)).round() as i32;
        d.clamp(depth_min, depth_max)
    } as usize;

    // initialize root alpha/beta for potential root-level cutoffs
    let mut alpha = MIN_EVAL_VALUE + 1;
    let beta = MAX_EVAL_VALUE - 1;

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

    // First: search one move serially (YBWC-lite) to seed bounds and provide good ordering
    let (first_from, first_to) = root_moves[0];
    // Use make/unmake on a single temporary board instead of cloning per move
    let mut tmp_first = board.clone();
    let undo_first = make_move_simple(&mut tmp_first, first_from, first_to);
    let mut first_tt = TranspositionTable::new_with_default_size();
    first_tt.next_age();
    let first_score_raw = if effective_depth <= 1 {
        evaluate_position(&tmp_first)
    } else {
        alphabeta(&mut tmp_first, opposite_color(active_color), effective_depth - 1, alpha, beta, 1, &mut first_tt)
    };
    // restore board
    unmake_move_simple(&mut tmp_first, undo_first);
    // Apply root-only adjustments identical to the original code
    let mut first_adjusted = first_score_raw + root_move_bonus(&board, first_from, first_to, active_color);
    if let Some(captured) = board.get(first_to.0, first_to.1) {
        use crate::piece::pieces::PieceType::*;
        let cap_val = match captured.get_type() {
            Pawn => 100, Knight => 320, Bishop => 330, Rook => 500, Queen => 900, King => 0,
        };
        first_adjusted += cap_val / 10;
    }
    // Inject random evaluation noise based on strength (weaker -> more noise)
    let sigma = strength_noise_sigma(ps as usize);
    if sigma > 0 {
        let n: i32 = rng().random_range(-sigma..=sigma);
        first_adjusted += n;
    }

    // Repetition-avoidance at root: if making this move would cause a 3-fold repetition
    // and the side to move currently stands better than a draw, penalize strongly.
    {
        let is_capture = board.get(first_to.0, first_to.1).is_some();
        let mut gs = game_state; // GameState is Copy
        let mut promote: Option<Piece> = None;
        // naive promotion to queen if a pawn reaches last rank
        if let Some(p) = gs.board().get(first_from.0, first_from.1) {
            if p.get_type() == PieceType::Pawn {
                if (active_color == Color::White && first_to.0 == 7) || (active_color == Color::Black && first_to.0 == 0) {
                    promote = Some(Piece::new(PieceType::Queen, active_color));
                }
            }
        }
        if PieceMover::move_piece(&mut gs, first_from, first_to, is_capture, promote) {
            gs.switch_player_turn();
            let fen = game_state_to_fen_string(gs);
            let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
            let count = history.fen_repetition_count(&truncated);
            let side_adv = if active_color == Color::White { first_adjusted } else { -first_adjusted };
            if count >= 2 && side_adv > 0 {
                // strong penalty to avoid draw when better
                first_adjusted -= if active_color == Color::White { 50_000 } else { -50_000 };
            }
        }
    }

    // Emit info for the first (seed) move
    let first_pv = build_pv_for_root(board, active_color, first_from, first_to, &first_tt, effective_depth);
    emit_info(first_from, first_to, first_adjusted, effective_depth, first_pv);

    // Update alpha/beta according to side to move
    if active_color == Color::White { if first_score_raw > alpha { alpha = first_score_raw; } }

    // Collect scored moves
    let mut move_table: Vec<((usize, usize), (usize, usize), i32)> = Vec::with_capacity(root_moves.len());
    move_table.push((first_from, first_to, first_adjusted));

    // Remaining moves
    let rest = &root_moves[1..];

    let _results: Vec<_> = rest.par_iter()
        .map(|&(from, to)| {
            // For parallel evals, each task uses its own temporary board copy with make/unmake
            let mut simulation_board = board.clone();
            let u = make_move_simple(&mut simulation_board, from, to);
            let mut local_tt = TranspositionTable::new_with_default_size();
            local_tt.next_age();
            let search_score = if effective_depth <= 1 {
                evaluate_position(&simulation_board)
            } else {
                alphabeta(&mut simulation_board, opposite_color(active_color), effective_depth - 1, alpha, beta, 1, &mut local_tt)
            };
            // unmake for cleanliness (not strictly required since board goes out of scope)
            unmake_move_simple(&mut simulation_board, u);
            let mut adjusted_score = search_score + root_move_bonus(&board, from, to, active_color);
            if let Some(captured) = board.get(to.0, to.1) {
                use crate::piece::pieces::PieceType::*;
                let cap_val = match captured.get_type() {
                    Pawn => 100, Knight => 320, Bishop => 330, Rook => 500, Queen => 900, King => 0,
                };
                adjusted_score += cap_val / 10;
            }
            let sigma = strength_noise_sigma(ps as usize);
            if sigma > 0 { let n: i32 = rng().random_range(-sigma..=sigma); adjusted_score += n; }
            // repetition-avoidance at root for parallel moves
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
                    let side_adv = if active_color == Color::White { adjusted_score } else { -adjusted_score };
                    if count >= 2 && side_adv > 0 {
                        adjusted_score -= if active_color == Color::White { 50_000 } else { -50_000 };
                    }
                }
            }
            // Emit info for each evaluated root move
            let pv = build_pv_for_root(board, active_color, from, to, &local_tt, effective_depth);
            emit_info(from, to, adjusted_score, effective_depth, pv);
            (from, to, adjusted_score)
        })
        .collect();
    move_table.extend(_results);

    if move_table.is_empty() { return None; }

    let mut sorted_moves = sort_moves_on_score_asc(&mut move_table);
    if active_color == Color::White { sorted_moves.reverse(); }

    if playing_strength >= 1000 {
        let bm = &sorted_moves.first().unwrap();
        Some((bm.0, bm.1, bm.2, effective_depth))
    } else {
        // When selecting based on strength randomness, we still want to report the chosen move's score
        if let Some((from, to)) = select_move_based_using_strength(&sorted_moves, playing_strength) {
            // Find the associated score in sorted_moves (there will be exactly one matching entry)
            if let Some((_, _, sc)) = sorted_moves.iter().find_map(|e| if e.0 == from && e.1 == to { Some((e.0, e.1, e.2)) } else { None }) {
                Some((from, to, sc, effective_depth))
            } else {
                // Fallback: if not found (shouldn't happen), use 0 score
                Some((from, to, 0, effective_depth))
            }
        } else {
            None
        }
    }
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

// Alpha-beta pruning search. Returns evaluation in centipawns (positive is better for White).
fn alphabeta(board: &mut Board, to_move: Color, depth: usize, mut alpha: i32, mut beta: i32, ply: i32, tt: &mut TranspositionTable) -> i32 {
    // Count every node we enter
    bump_node();

    // print board
    //println!("alpha-beta\n{}", board.get_board_display_string(None));

    if depth == 0 {
        /*let score = evaluate_position(board);
        //println!("score: {}", score);
        return score;*/
        // At leaf: switch to quiescence to avoid horizon effects
        return qsearch(board, to_move, alpha, beta);
    }

    // TT probe
    let key = compute_zobrist(&*board, to_move);
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
    // Basic move ordering: after TT move, sort captures first using MVV-LVA heuristic
    if moves.len() > 1 {
        // Stable-partition: keep index 0 (possibly TT move) in place, sort the tail
        let (head, tail) = moves.split_at_mut(1);
        let board_ref = &*board;
        tail.sort_by_key(|&(from, to)| -move_score_mvv_lva(board_ref, from, to));
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
    let value = if to_move == Color::White {
        let mut value = MIN_EVAL_VALUE;
        for (from, to) in moves.into_iter() {
            let u = make_move_simple(board, from, to);
            let score = alphabeta(board, Color::Black, depth - 1, alpha, beta, ply + 1, tt);
            unmake_move_simple(board, u);
            if score > value { value = score; }
            if value > alpha { alpha = value; best_from_to = Some((from, to)); }
            if alpha >= beta { break; }
        }
        value
    } else {
        let mut value = MAX_EVAL_VALUE;
        for (from, to) in moves.into_iter() {
            let u = make_move_simple(board, from, to);
            let score = alphabeta(board, Color::White, depth - 1, alpha, beta, ply + 1, tt);
            unmake_move_simple(board, u);
            if score < value { value = score; }
            if value < beta { beta = value; best_from_to = Some((from, to)); }
            if alpha >= beta { break; }
        }
        value
    };

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
fn qsearch(board: &mut Board, to_move: Color, mut alpha: i32, beta: i32) -> i32 {
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
                }
            }
            let u = make_move_simple(board, from, to);
            let score = qsearch(board, Color::Black, a, bnd);
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
                }
            }
            let u = make_move_simple(board, from, to);
            let score = qsearch(board, Color::White, a, bnd);
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
