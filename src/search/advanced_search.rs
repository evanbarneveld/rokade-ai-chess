use crate::board::Board;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{opposite_color, Color, Piece, PieceType};
use crate::search::locking::get_tt_mutex;
use crate::search::playing_strength::{select_move_based_using_strength, PLAYING_STRENGTH_MAX};
use crate::search::root_moves::{
    adjusted_root_eval_for_move, build_pv_for_root, evaluate_after_root_move, get_root_moves,
    hard_root_filter,
};
use crate::search::threading::init_rayon_pool_if_needed;
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::uci_feedback::emit_info;
use crate::search::zobrist::compute_zobrist;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;
use rayon::prelude::*;
pub(crate) use crate::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};

pub const SEARCH_ABORTED: i32 = MAX_EVAL_VALUE + 50000;
use crate::book::book::book_pick;
use crate::search::{is_parallel_search, Search};
use crate::board::san_move::convert_move_to_san;

pub const DEFAULT_SEARCH_DEPTH: usize = 15;
pub const MAX_SEARCH_DEPTH: usize = 20;

// Root parallelization settings
const ROOT_PARALLEL_MIN_DEPTH: usize = 6; // TODO not the default enable root parallel only from this depth
const ROOT_PARALLEL_MIN_MOVES: usize = 4; // and when at least this many root moves exist

const ORDER_BOOK_ENABLED: bool = true; // TODO not the default
const STRENGTH_MODE_ENABLED: bool = true; // TODO not the default

// Global toggle to enable/disable Zobrist hashing across the engine.
// When disabled, features relying on Zobrist keys (like TT and repetition checks)
// will be bypassed.
pub(crate) const ZOBRIST_HASHING_ENABLED: bool = true;

// Tie the transposition table to Zobrist hashing. Without Zobrist keys, TT is disabled.
pub(crate) const TRANSPOSITION_TABLE_ENABLED: bool = ZOBRIST_HASHING_ENABLED; // WARNING: Disabling TT can be 2–10x slower

pub(crate) const NULL_MOVE_PRUNING_ENABLED: bool = true;
pub(crate) const SEE_FILTERING_ENABLED: bool = true;
pub(crate) const ASPIRATION_WINDOWS_ENABLED: bool = true;

pub(crate) const QUIESCENCE_ENABLED: bool = true ; // TODO Disabling may cause horizon effects
pub(crate) const QSEE_PRUNING_ENABLED: bool = true; // SEE-based pruning inside qsearch
pub(crate) const MVV_LVA_ENABLED: bool = true; // Capture ordering heuristic
pub(crate) const LMR_ENABLED: bool = true; // Late Move Reductions
pub(crate) const ID_ITERATIONS_ENABLED: bool = true; // Iterative deepening loop

// Iterative deepening aspiration window (in centipawns)
// With a stronger, more stable evaluator we can start tighter and cap lower.
const ASP_WINDOW_INIT_CP: i32 = 30; // initial aspiration half-window
const ASP_WINDOW_MAX_CP: i32 = 400; // maximum expanded half-window

// Root repetition-avoidance bias when a move would immediately create 3-fold
const REP_AVOIDANCE_BIAS_CP: i32 = 50_000;
pub const MAX_PLAYING_STRENGTH: usize = 1000;
pub const DEFAULT_MOVE_TIME_FOR_STRENGTH_MODE_PLAY: usize = 3000usize;

// Provide a simple implementor of the `Search` trait that forwards to this module's function.
// This keeps existing callers of the free function intact while enabling trait-based use.
pub struct AdvancedSearch;

impl Search for AdvancedSearch {
    fn find_best_move(
        game_state: &GameState,
        history: &History,
        search_depth: usize,
        playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), i32, usize)> {
        find_best_move(
            game_state,
            history,
            search_depth,
            playing_strength,
        )
    }
}

/// Find the best move for the given game state, the search_depth, and the playing_strength
/// returns the evaluated score (in centipawns) for the selected move
/// and the effective Search depth that was actually used internally.

pub fn find_best_move(
    game_state: &GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), i32, usize)> {
    init_rayon_pool_if_needed();

    // Persistent Transposition Table across searches: initialize once and reuse.
    // We keep it behind a Mutex to allow mutable access in this serial root Search.

    let tt_mutex = get_tt_mutex();

    // collect all legal moves for the side to move
    let board = game_state.board();
    let active_color = game_state.active_color();
    let gen_moves = find_all_valid_moves(game_state);

    //dump_all_valid_moves(game_state, active_color, true);

    if gen_moves.is_empty() {
        return None;
    }

    // Opening book: if we have a book move in early game, play it immediately.
    // Limit to first ~8 full moves to avoid forcing book deep into middlegame.
    if ORDER_BOOK_ENABLED {
        if game_state.full_move_number() <= 8 {
            if let Some((bf, bt)) = book_pick(game_state) {
                return Some((bf, bt, 0, 0));
            }
        }
    }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if search_depth == 0 { 1 } else { search_depth };

    // Map playing_strength [1..1000] to an effective depth to intentionally weaken play at low strengths.
    // Rough mapping: at ~300 strength, cap to ~3 ply; at 1000 keep requested depth.
    let ps = if playing_strength == 0 {
        1
    } else {
        playing_strength.min(PLAYING_STRENGTH_MAX)
    } as i32;

    let effective_depth = search_depth;

    // Root-level hard 3-fold avoidance: filter out any root move that would create
    // a third occurrence of the same position (per truncated FEN used in History).
    // If filtering removes all moves (e.g., only repetition saves a loss), fall back to all moves.
    // Map generated triples to pairs for existing root pipeline (duplicates preserved)
    let moves: Vec<((usize, usize), (usize, usize))> = gen_moves
        .iter()
        .map(|(f, t, _)| (*f, *t))
        .collect();

    let root_moves: Vec<((usize, usize), (usize, usize))> = {
        let mut v = Vec::with_capacity(moves.len());

        get_root_moves(game_state, history, board, active_color, &moves, &mut v);

        // Hard root filter: drop unsafe queen moves (SEE<0) and unsafe minor-piece non-check sacs
        // (SEE<=SEE_MINOR_SAC_THRESHOLD_CP and not giving check)
        // If filtering removes all, keep original set.
        let mut base: Vec<((usize, usize), (usize, usize))> = if SEE_FILTERING_ENABLED && !v.is_empty() {
            let mut filtered: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(v.len());
            hard_root_filter(board, active_color, &mut v, &mut filtered);
            if filtered.is_empty() { v } else { filtered }
        } else {
            v
        };

        // Additional opening hard filter at root: drop quiet queen moves that do not give check
        // (captures and checks preserved). This is opening-only and keeps tactics intact.
        // Keeps at least one move if it would otherwise drop all.
        let mut phase_for_scale = 0i32;
        for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
            phase_for_scale += match p.get_type() { PieceType::Knight|PieceType::Bishop => 1, PieceType::Rook => 2, PieceType::Queen => 4, _ => 0 };
        }}}
        phase_for_scale = phase_for_scale.clamp(0,24);
        if phase_for_scale >= 16 {
            let mut kept: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(base.len());
            for &(f, t) in &base {
                let p = board.get(f.0, f.1);
                let is_cap = board.get(t.0, t.1).is_some();
                let mut btmp = board.clone();
                if let Some(mp) = p { btmp.set(t.0, t.1, Some(mp)); btmp.set(f.0, f.1, None); }
                let gives_check = if p.is_some() { btmp.is_side_in_check(opposite_color(active_color)) } else { false };
                let is_quiet_queen = matches!(p, Some(pc) if pc.get_type()==PieceType::Queen) && !is_cap;
                if !(is_quiet_queen && !gives_check) {
                    kept.push((f,t));
                }
            }
            if !kept.is_empty() { base = kept; }
        }

        // Heuristic ordering at root: prioritize checking moves, then captures by MVV, then others.
        base.sort_by(|&(f1,t1), &(f2,t2)| {
            use crate::piece::pieces::{piece_value_cp};
            let mut b1 = board.clone();
            let mut b2 = board.clone();
            let p1 = board.get(f1.0, f1.1);
            let p2 = board.get(f2.0, f2.1);
            let cap1 = board.get(t1.0, t1.1);
            let cap2 = board.get(t2.0, t2.1);
            if let Some(mp) = p1 { b1.set(t1.0, t1.1, Some(mp)); b1.set(f1.0, f1.1, None); }
            if let Some(mp) = p2 { b2.set(t2.0, t2.1, Some(mp)); b2.set(f2.0, f2.1, None); }
            let check1 = if p1.is_some() { b1.is_side_in_check(opposite_color(active_color)) } else { false };
            let check2 = if p2.is_some() { b2.is_side_in_check(opposite_color(active_color)) } else { false };
            let key1 = (check1 as i32) * 10000 + cap1.map(|pc| piece_value_cp(pc.get_type())).unwrap_or(0);
            let key2 = (check2 as i32) * 10000 + cap2.map(|pc| piece_value_cp(pc.get_type())).unwrap_or(0);
            key2.cmp(&key1) // descending
        });
        base
    };

    // Iterative Deepening + Aspiration windows at root (serial evaluation for stability)
    // Reuse persistent TT
    let mut tt = tt_mutex.lock().unwrap();
    let base_hmc = game_state.half_move_clock();
    let mut _last_score: i32 = 0;
    let mut chosen: Option<((usize, usize), (usize, usize), i32, usize)> = None;
    let mut window: i32 = ASP_WINDOW_INIT_CP; // cp

    //eprintln!("[root] starting ID; eff_depth={} root_moves={} window={}",
    //          effective_depth, root_moves.len(), window);

    if ID_ITERATIONS_ENABLED {
        for depth_now in 1..=effective_depth {

            //eprintln!("[root] depth_now={} (pre-asp) last_score={} window={}", depth_now, last_score, window);

            tt.next_age();
            let ((bf, bt), best_adj, best_raw) = if ASPIRATION_WINDOWS_ENABLED {
                probe_with_aspiration(
                    &board,
                    active_color,
                    &root_moves,
                    depth_now,
                    _last_score,
                    &mut window,
                    &mut tt,
                    base_hmc,
                    ps,
                    game_state,
                    history,
                )
            } else {
                // Full-width single probe without aspiration
                evaluate_root_for_bounds(
                    &board,
                    active_color,
                    &root_moves,
                    depth_now,
                    MIN_EVAL_VALUE + 1,
                    MAX_EVAL_VALUE - 1,
                    &mut tt,
                    base_hmc,
                    ps,
                    game_state,
                    history,
                )
            };

            if best_raw == SEARCH_ABORTED {
                break;
            }

            _last_score = best_raw;
            // Emit PV/info for this iteration, including TT hashfull permille
            let pv = build_pv_for_root(board, active_color, bf, bt, &tt, depth_now);
            let hf = tt.hashfull_permille();
            emit_info(bf, bt, best_adj, depth_now, pv, hf);
            chosen = Some((bf, bt, best_adj, depth_now));
        }
    } else {
        // Single-depth Search without iterative deepening
        let depth_now = effective_depth;
        tt.next_age();
        let ((bf, bt), best_adj, best_raw) = if ASPIRATION_WINDOWS_ENABLED {
            probe_with_aspiration(
                &board,
                active_color,
                &root_moves,
                depth_now,
                _last_score,
                &mut window,
                &mut tt,
                base_hmc,
                ps,
                game_state,
                history,
            )
        } else {
            evaluate_root_for_bounds(
                &board,
                active_color,
                &root_moves,
                depth_now,
                MIN_EVAL_VALUE + 1,
                MAX_EVAL_VALUE - 1,
                &mut tt,
                base_hmc,
                ps,
                game_state,
                history,
            )
        };
        _last_score = best_raw;
        let pv = build_pv_for_root(board, active_color, bf, bt, &tt, depth_now);
        let hf = tt.hashfull_permille();
        // Always report UCI scores from White's perspective
        let white_persp_score = if active_color == Color::Black { -best_adj } else { best_adj };
        emit_info(bf, bt, white_persp_score, depth_now, pv, hf);
        chosen = Some((bf, bt, best_adj, depth_now));
    }

    // Final selection based on playing_strength from the last iteration
    if let Some((bf, bt, sc, used_depth)) = chosen {
        if STRENGTH_MODE_ENABLED && playing_strength < MAX_PLAYING_STRENGTH {
            // Re-evaluate top K moves for stochastic selection at final depth
            let mut scored: Vec<((usize, usize), (usize, usize), i32)> = Vec::new();
            for &(from, to) in &root_moves {
                let (sr, is_capture, moved_is_pawn) = evaluate_after_root_move(
                    &board,
                    active_color,
                    from,
                    to,
                    effective_depth,
                    MIN_EVAL_VALUE + 1,
                    MAX_EVAL_VALUE - 1,
                    &mut tt,
                    base_hmc,
                );
                let adj = adjusted_root_eval_for_move(
                    &board,
                    active_color,
                    from,
                    to,
                    base_hmc,
                    sr,
                    is_capture,
                    moved_is_pawn,
                    ps,
                );
                scored.push((from, to, adj));
            }

            sort_moves_on_score_asc(&mut scored);
            if active_color == Color::White {
                scored.reverse();
            }

            if let Some((from, to)) = select_move_based_using_strength(&scored, playing_strength) {
                let sc = scored
                    .iter()
                    .find(|e| e.0 == from && e.1 == to)
                    .map(|e| e.2)
                    .unwrap_or(sc);
                Some((from, to, sc, used_depth))
            } else {
                Some((bf, bt, sc, used_depth))
            }
        } else {
            Some((bf, bt, sc, used_depth))
        }
    } else {
        None
    }
}

/// Debug helper: rank root moves for the current position, returning (SAN, adjusted_score, raw_score)
/// Sorted by side to move preference (White: descending; Black: ascending).
pub fn debug_rank_root_moves(
    game_state: &GameState,
    history: &History,
    depth: usize,
    playing_strength: usize,
) -> Vec<(String, i32, i32)> {
    let board = game_state.board();
    let active_color = game_state.active_color();
    let base_hmc = game_state.half_move_clock();
    let gen_moves = find_all_valid_moves(game_state);
    let moves: Vec<((usize, usize), (usize, usize))> = gen_moves.iter().map(|(f,t,_)| (*f,*t)).collect();

    let mut v = Vec::with_capacity(moves.len());
    get_root_moves(game_state, history, board, active_color, &moves, &mut v);
    let mut filtered: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(v.len());
    hard_root_filter(board, active_color, &mut v, &mut filtered);
    let root = if filtered.is_empty() { v } else { filtered };

    // No TT needed for a one-shot evaluation per move here
    let mut tt = get_tt_mutex().lock().unwrap();
    let mut out: Vec<(String,i32,i32)> = Vec::with_capacity(root.len());
    for (from, to) in root {
        let (raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
            board,
            active_color,
            from,
            to,
            depth.max(1),
            MIN_EVAL_VALUE + 1,
            MAX_EVAL_VALUE - 1,
            &mut tt,
            base_hmc,
        );
        let adj = adjusted_root_eval_for_move(
            board,
            active_color,
            from,
            to,
            base_hmc,
            raw,
            is_capture,
            moved_is_pawn,
            playing_strength as i32,
        );
        let san = convert_move_to_san(*game_state, Some((from,to))).unwrap_or_else(|| {
            format!("{}{}", crate::piece::as_square_str(from), crate::piece::as_square_str(to))
        });
        out.push((san, adj, raw));
    }
    if active_color == Color::White {
        out.sort_by(|a,b| b.1.cmp(&a.1));
    } else {
        out.sort_by(|a,b| a.1.cmp(&b.1));
    }
    out
}

pub fn find_all_valid_moves(
    game_state: &GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let board = game_state.board();
    let active_color = game_state.active_color();

    // iterate all squares and collect legal moves for the active color
    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color {
                continue;
            }

            for tr in 0..8 {
                for tc in 0..8 {
                    let from = (r, c);
                    let to = (tr, tc);
                    if from == to {
                        continue;
                    }

                    let target_piece_is_some = board.get(tr, tc).is_some();

                    // basic board-level validation (ownership, capture flags, bounds)
                    let is_capture = target_piece_is_some
                        || (piece.get_type() == PieceType::Pawn
                            && game_state.en_passant_target().is_some()
                            && to == game_state.en_passant_target().unwrap());
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !game_state.move_from_and_to_validation_check(
                        from,
                        to,
                        active_color,
                        is_capture,
                        is_pawn_move,
                        game_state.en_passant_target(),
                    ) {
                        continue;
                    }

                    // Use full GameState-aware move application to validate legality, covering:
                    // - pins/check (including en passant discovered checks)
                    // - castling rights and rook/king path clearance
                    // - en passant captures
                    // - promotions (try all promotion piece types)
                    let mut gs = *game_state;
                    let is_pawn_promotion = piece.get_type() == PieceType::Pawn
                        && ((active_color == Color::White && tr == 7)
                            || (active_color == Color::Black && tr == 0));

                    if is_pawn_promotion {
                        // Try all legal promotion pieces: Queen, Rook, Bishop, Knight
                        // Note: We push the same (from,to) four times if all are legal,
                        // so perft and generators can count distinct promotions separately.
                        let promo_types = [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ];
                        for pt in promo_types.iter() {
                            let mut gs_var = gs; // work from the same pre-move state
                            let promo_piece = Some(Piece::new(*pt, active_color));
                            if PieceMover::move_piece(&mut gs_var, from, to, is_capture, promo_piece)
                            {
                                let ch = match pt {
                                    PieceType::Queen => Some('q'),
                                    PieceType::Rook => Some('r'),
                                    PieceType::Bishop => Some('b'),
                                    PieceType::Knight => Some('n'),
                                    _ => None,
                                };
                                result.push((from, to, ch));
                            }
                        }
                    } else {
                        if PieceMover::move_piece(&mut gs, from, to, is_capture, None) {
                            result.push((from, to, None));
                        }
                    }
                }
            }
        }
    }
    result
}

// Lightweight move for perft: includes capture flag and promotion marker.
#[derive(Clone, Copy, Debug)]
pub struct PerftMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub is_capture: bool,
    pub promo: Option<char>,
}

/// Fill `out` with all legal moves for the active side, including capture flag and promotion marker.
/// This mirrors `find_all_valid_moves` but avoids allocating a new Vec every call and returns flags.
pub fn find_all_valid_moves_into_perft(game_state: &GameState, out: &mut Vec<PerftMove>) {
    out.clear();
    let board = game_state.board();
    let active_color = game_state.active_color();

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
                    let is_capture = target_piece_is_some
                        || (piece.get_type() == PieceType::Pawn
                            && game_state.en_passant_target().is_some()
                            && to == game_state.en_passant_target().unwrap());
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !game_state.move_from_and_to_validation_check(
                        from, to, active_color, is_capture, is_pawn_move, game_state.en_passant_target(),
                    ) { continue; }

                    let mut gs = *game_state;
                    let is_pawn_promotion = piece.get_type() == PieceType::Pawn
                        && ((active_color == Color::White && tr == 7)
                            || (active_color == Color::Black && tr == 0));

                    if is_pawn_promotion {
                        let promo_types = [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ];
                        for pt in promo_types.iter() {
                            let mut gs_var = gs;
                            let promo_piece = Some(Piece::new(*pt, active_color));
                            if PieceMover::move_piece(&mut gs_var, from, to, is_capture, promo_piece) {
                                let ch = match pt {
                                    PieceType::Queen => Some('q'),
                                    PieceType::Rook => Some('r'),
                                    PieceType::Bishop => Some('b'),
                                    PieceType::Knight => Some('n'),
                                    _ => None,
                                };
                                out.push(PerftMove { from, to, is_capture, promo: ch });
                            }
                        }
                    } else {
                        if PieceMover::move_piece(&mut gs, from, to, is_capture, None) {
                            out.push(PerftMove { from, to, is_capture, promo: None });
                        }
                    }
                }
            }
        }
    }
}

/// Dump all legal moves for the given side from the current board, formatted as SAN or coordinate pairs.
/// This is intended for debugging/tests. It returns a single string with moves separated by spaces.
/// By default it uses simple coordinate notation like e2e4; when `to_san` is true and a GameState is
/// provided, it will attempt to convert to SAN.
pub fn _dump_all_valid_moves(
    game_state: &GameState,
    to_san: bool,
) {
    use crate::board::san_move::convert_move_to_san;
    let moves = find_all_valid_moves(game_state);
    if moves.is_empty() {
        println!("No moves");
        return;
    }
    if to_san {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, _promo) in moves {
            if let Some(s) = convert_move_to_san(*game_state, Some((from, to))) {
                parts.push(s);
            } else {
                // fallback to coord if SAN conversion fails
                let s = format!(
                    "{}{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char,
                    _promo.unwrap_or('\0')
                );
                parts.push(s.trim_end_matches('\0').to_string());
            }
        }
        println!("{}", parts.join(" "));
        return;
    } else {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, promo) in moves {
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
            parts.push(s);
        }
        println!("{}", parts.join(" "));
    }
}

// Sorts the move table by score in ascending order, in-place.
fn sort_moves_on_score_asc(
    move_table: &mut Vec<((usize, usize), (usize, usize), i32)>,
) {
    move_table.sort_by_key(|m| m.2);
}

#[inline]
fn reorder_with_tt_hint(
    ordered: &mut Vec<((usize, usize), (usize, usize))>,
    tt: &TranspositionTable,
    board: &Board,
    side: Color,
) {
    if let Some(entry) = tt.probe(compute_zobrist(board, side)) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = ordered.iter().position(|m| *m == bm) {
            let first = ordered.remove(pos);
            ordered.insert(0, first);
        }
    }
}

#[inline]
fn aspiration_bounds_for_depth(depth_now: usize, last_score: i32, window: i32) -> (i32, i32) {
    // Use full window for very shallow depths where last_score is unreliable
    if depth_now <= 3 {
        (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1)
    } else {
        (
            (last_score - window).max(MIN_EVAL_VALUE + 1),
            (last_score + window).min(MAX_EVAL_VALUE - 1),
        )
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn evaluate_root_for_bounds(
    board: &Board,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize))>,
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
    ps: i32,
    game_state: &GameState,
    history: &History,
) -> (((usize, usize), (usize, usize)), i32, i32) {
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    let mut best_score_raw = if active_color == Color::White {
        MIN_EVAL_VALUE
    } else {
        MAX_EVAL_VALUE
    };
    let mut best_adjusted = best_score_raw;

    // Order: if TT has a move at root, try to place it first, then apply light opening-aware tie-breakers
    let mut ordered: Vec<((usize, usize), (usize, usize))> = root_moves.iter().copied().collect();
    reorder_with_tt_hint(&mut ordered, tt, board, active_color);

    // Opening-aware tiny reordering: demote quiet queen moves; promote minor development and castling
    // Keep this extremely small so as not to override tactical ordering.
    // Apply only at very shallow plies (root) and more in opening phase.
    let phase_for_scale = {
        // Phase proxy: count heavy/minors on board similar to evaluator's game_phase; fallback small constant
        let mut phase = 0i32;
        for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
            phase += match p.get_type() { PieceType::Knight|PieceType::Bishop => 1, PieceType::Rook => 2, PieceType::Queen => 4, _ => 0 };
        }}}
        if phase < 0 { 0 } else if phase > 24 { 24 } else { phase }
    } as i32;
    if depth_now >= 1 {
        ordered.sort_by(|&(f1,t1), &(f2,t2)| {
            let score1 = root_move_order_bias(board, active_color, f1, t1, phase_for_scale);
            let score2 = root_move_order_bias(board, active_color, f2, t2, phase_for_scale);
            score2.cmp(&score1) // higher bias first
        });
    }

    //eprintln!("[root-bounds] depth={} ordered={} a={} b={} parallel?={}",
    //          depth_now, ordered.len(), a, b,
    //         (depth_now >= ROOT_PARALLEL_MIN_DEPTH && ordered.len() >= ROOT_PARALLEL_MIN_MOVES));

    let enable_parallel = is_parallel_search()
        && depth_now >= ROOT_PARALLEL_MIN_DEPTH
        && ordered.len() >= ROOT_PARALLEL_MIN_MOVES;
    if enable_parallel {
        //eprintln!("Parallel root Search enabled");
        // 1) Search the first (best-ordered) move serially to establish PV and bounds
        let &(pv_from, pv_to) = ordered.first().unwrap();
        {
            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                board,
                active_color,
                pv_from,
                pv_to,
                depth_now,
                a,
                b,
                tt,
                base_hmc,
            );

            if score_raw == SEARCH_ABORTED {
                return (((0, 0), (0, 0)), SEARCH_ABORTED, SEARCH_ABORTED);
            }

            let adjusted = adjusted_root_eval_for_move(
                board,
                active_color,
                pv_from,
                pv_to,
                base_hmc,
                score_raw,
                is_capture,
                moved_is_pawn,
                ps,
            );

            best_from_to = Some((pv_from, pv_to));
            best_adjusted = adjusted;
            best_score_raw = score_raw;
        }

        // 2) Search the remaining moves in parallel with per-task local TT to avoid contention
        // reuse shared board reference in parallel (read-only access)
        let base_hmc_loc = base_hmc;
        let a_loc = a;
        let b_loc = b;
        let side = active_color;
        let results = ordered[1..]
            .par_iter()
            .map(|&(from, to)| {
                // local TT per task
                let mut local_tt = TranspositionTable::new_with_default_size();
                let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                    board,
                    side,
                    from,
                    to,
                    depth_now,
                    a_loc,
                    b_loc,
                    &mut local_tt,
                    base_hmc_loc,
                );

                if score_raw == SEARCH_ABORTED {
                    return (from, to, SEARCH_ABORTED, SEARCH_ABORTED);
                }

                // Root adjustments (skip repetition-history check to keep parallel code simple)
                let adjusted = adjusted_root_eval_for_move(
                    board,
                    side,
                    from,
                    to,
                    base_hmc_loc,
                    score_raw,
                    is_capture,
                    moved_is_pawn,
                    ps,
                );
                (from, to, adjusted, score_raw)
            })
            .reduce(
                || {
                    // Identity: invalid move placeholder not used; return extreme sentinel
                    (
                        (0usize, 0usize),
                        (0usize, 0usize),
                        if side == Color::White {
                            MIN_EVAL_VALUE
                        } else {
                            MAX_EVAL_VALUE
                        },
                        if side == Color::White {
                            MIN_EVAL_VALUE
                        } else {
                            MAX_EVAL_VALUE
                        },
                    )
                },
                |acc, x| {
                    let better = if side == Color::White {
                        x.2 > acc.2
                    } else {
                        x.2 < acc.2
                    };
                    if better { x } else { acc }
                },
            );

        // Update best with parallel results if better
        let (pf, pt, padj, praw) = results;
        if praw == SEARCH_ABORTED {
            return (((0, 0), (0, 0)), SEARCH_ABORTED, SEARCH_ABORTED);
        }
        // Ignore identity placeholder
        if !(pf == (0, 0) && pt == (0, 0)) {
            let better = if active_color == Color::White {
                padj > best_adjusted
            } else {
                padj < best_adjusted
            };
            if better {
                best_from_to = Some((pf, pt));
                best_adjusted = padj;
                best_score_raw = praw;
            }
        }
    } else {
        // Search sequentially over root moves
        //eprintln!(
        //    "[root-serial] depth={} scanning {} moves with a={} b={}",
        //    depth_now,
        //    ordered.len(),
        //    a,
        //    b
        //);
        for &(from, to) in &ordered {

            //eprintln!("[root-serial] try mv={:?}->{:?}", from, to);

            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                board,
                active_color,
                from,
                to,
                depth_now,
                a,
                b,
                tt,
                base_hmc,
            );

            if score_raw == SEARCH_ABORTED {
                return (((0, 0), (0, 0)), SEARCH_ABORTED, SEARCH_ABORTED);
            }

            //eprintln!(
            //    "[root-serial] mv={:?}->{:?} raw={} (alpha={}, beta={})",
            //    (from, to).0,
            //    (from, to).1,
            //    score_raw,
            //    a,
            //    b
            //);

            // Adjust score for root-only heuristics
            let mut adjusted = adjusted_root_eval_for_move(
                board,
                active_color,
                from,
                to,
                base_hmc,
                score_raw,
                is_capture,
                moved_is_pawn,
                ps,
            );
            // repetition-avoidance at root
            adjusted = apply_repetition_avoidance_bias(
                adjusted,
                game_state,
                history,
                board,
                active_color,
                from,
                to,
            );

            // Track best
            let better = if active_color == Color::White {
                adjusted > best_adjusted
            } else {
                adjusted < best_adjusted
            };

            //eprintln!(
            //  "[root-serial] adj={} best_adj_so_far={} best_raw_so_far={}",
            //    adjusted,
            //    best_adjusted,
            //    best_score_raw
            //);

            if better || best_from_to.is_none() {
                best_from_to = Some((from, to));
                best_adjusted = adjusted;
                best_score_raw = score_raw;
            }
            // Aspiration cutoffs help ordering mid-loop too
            if active_color == Color::White && score_raw >= b {
                //eprintln!("[root-serial] cutoff WHITE raw={} >= beta={}, break", score_raw, b);
                break;
            }
            if active_color == Color::Black && score_raw <= a {
                //eprintln!("[root-serial] cutoff BLACK raw={} <= alpha={}, break", score_raw, a);
                break;
            }
        }
    }

    //eprintln!(
    //    "[root-serial] RETURN depth={} mv={:?} best_raw={} best_adj={}",
    //    depth_now,
    //    best_from_to.unwrap(),
    //    best_score_raw,
    //    best_adjusted
    //);

    (best_from_to.unwrap(), best_adjusted, best_score_raw)
}

// Tiny heuristic score for root ordering; positive favors earlier search
#[inline]
fn root_move_order_bias(board: &Board, side: Color, from: (usize,usize), to: (usize,usize), phase: i32) -> i32 {
    // scale 0..24 -> 0..24
    let scale = phase.clamp(0,24);
    let mut bias: i32 = 0;
    // Identify moved piece
    let piece = match board.get(from.0, from.1) { Some(p) if p.get_color()==side => p, _ => return 0 };
    let is_capture = board.get(to.0, to.1).is_some();

    // Prefer castling
    if piece.get_type()==PieceType::King {
        let dr = if side==Color::White { 0usize } else { 7usize };
        if from.0==dr && (to.1==6 || to.1==2) { bias += (10 * scale) / 24; }
    }

    // Prefer minor development from back rank
    if piece.get_type()==PieceType::Knight || piece.get_type()==PieceType::Bishop {
        let back = if side==Color::White { 0usize } else { 7usize };
        if from.0 == back {
            bias += (8 * scale) / 24; // slightly stronger nudge
        }
    }

    // Demote quiet queen moves in opening at root (unless capture)
    if piece.get_type()==PieceType::Queen && !is_capture {
        // Light demotion; guards: if move gives check we'll discover tactically later
        let mut demote = 9; // base demotion strength
        // Extra demotion if position is underdeveloped (>=2 minors on back rank)
        let back_r = if side==Color::White { 0usize } else { 7usize };
        let mut undeveloped = 0;
        for fc in 0..8 {
            if let Some(p) = board.get(back_r, fc) {
                if p.get_color()==side {
                    if matches!(p.get_type(), PieceType::Knight | PieceType::Bishop) { undeveloped += 1; }
                }
            }
        }
        if undeveloped >= 3 { demote += 6; } else if undeveloped >= 2 { demote += 3; }
        // Extra demotion for big queen sorties (long leaps) in the opening
        let manhattan = (from.0 as i32 - to.0 as i32).abs() + (from.1 as i32 - to.1 as i32).abs();
        if manhattan >= 3 { demote += 4; }
        // Extra demotion for advancing deep (enemy side) without capture
        let deep_adv = match side { Color::White => to.0 >= 3, Color::Black => to.0 <= 4 };
        if deep_adv { demote += 3; }
        bias -= (demote * scale) / 24;
    }
    bias
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn probe_with_aspiration(
    board: &Board,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize))>,
    depth_now: usize,
    last_score: i32,
    window: &mut i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
    ps: i32,
    game_state: &GameState,
    history: &History,
) -> (((usize, usize), (usize, usize)), i32, i32) {
    let (mut a, mut b) = aspiration_bounds_for_depth(depth_now, last_score, *window);

    //eprintln!("[asp] depth={} init a={} b={} last={}", depth_now, a, b, last_score);

    let mut tried = 0;
    loop {
        tried += 1;

        //eprintln!("[asp] depth={} try={} a={} b={}", depth_now, tried, a, b);

        let (_mv, _best_adjusted, best_score_raw) = evaluate_root_for_bounds(
            board,
            active_color,
            root_moves,
            depth_now,
            a,
            b,
            tt,
            base_hmc,
            ps,
            game_state,
            history,
        );

        if best_score_raw == SEARCH_ABORTED {
            return (((0, 0), (0, 0)), SEARCH_ABORTED, SEARCH_ABORTED);
        }

        //eprintln!("[asp] result depth={} try={} raw={} adj={} mv={:?}->{:?}",
        //          depth_now, tried, best_score_raw, best_adjusted, mv.0, mv.1);

        // Check aspiration result
        if best_score_raw <= a {
            // fail-low: widen down

            //eprintln!("[asp] FAIL-LOW depth={} try={} raw={} <= a={}; expand window {}->{}",
            //          depth_now, tried, best_score_raw, a, *window, (*window * 2).min(ASP_WINDOW_MAX_CP));

            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            a = bounds.0;
            if tried < 3 { continue; }
        } else if best_score_raw >= b {
            // fail-high: widen up

            //eprintln!("[asp] FAIL-HIGH depth={} try={} raw={} >= b={}; expand window {}->{}",
            //          depth_now, tried, best_score_raw, b, *window, (*window * 2).min(ASP_WINDOW_MAX_CP));

            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            b = bounds.1;
            if tried < 3 { continue; }
        }
        // At this point we have tried a few widened windows but still failed to land inside bounds.
        // To ensure a stable PV update at this depth, fall back to a full-width search at once.
         {
            // Reset to the full window and a modest aspiration window for subsequent depths
            *window = (*window).max(ASP_WINDOW_INIT_CP);
            let (fa, fb) = (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1);
            let (mv2, best_adj2, best_raw2) = evaluate_root_for_bounds(
                board,
                active_color,
                root_moves,
                depth_now,
                fa,
                fb,
                tt,
                base_hmc,
                ps,
                game_state,
                history,
            );
            if best_raw2 == SEARCH_ABORTED {
                return (((0, 0), (0, 0)), SEARCH_ABORTED, SEARCH_ABORTED);
            }
            return (mv2, best_adj2, best_raw2);
        }
    }
}

#[inline]
fn apply_repetition_avoidance_bias(
    adjusted: i32,
    game_state: &GameState,
    history: &History,
    board: &Board,
    active_color: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    let mut adjusted = adjusted;
    let is_capture = board.get(to.0, to.1).is_some();
    let mut gs = *game_state; // Copy
    let mut promote: Option<Piece> = None;
    if let Some(p) = gs.board().get(from.0, from.1) {
        if p.get_type() == PieceType::Pawn {
            if (active_color == Color::White && to.0 == 7)
                || (active_color == Color::Black && to.0 == 0)
            {
                promote = Some(Piece::new(PieceType::Queen, active_color));
            }
        }
    }
    if PieceMover::move_piece(&mut gs, from, to, is_capture, promote) {
        gs.switch_player_turn();
        let fen = game_state_to_fen_string(gs);
        let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
        let count = history.fen_repetition_count(&truncated);
        let sa = if active_color == Color::White {
            adjusted
        } else {
            -adjusted
        };
        if count >= 2 && sa > 0 {
            adjusted -= if active_color == Color::White {
                REP_AVOIDANCE_BIAS_CP
            } else {
                -REP_AVOIDANCE_BIAS_CP
            };
        }
    }
    adjusted
}
