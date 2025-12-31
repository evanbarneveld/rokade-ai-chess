use crate::board::Board;
use crate::board::evaluator::evaluate_position;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{capture_value_cp, opposite_color, piece_value_cp, Color, Piece, PieceType};
use crate::search::tt::{TranspositionTable, decode_move};
use crate::search::zobrist::compute_zobrist;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;
use rand::{Rng, rng};
use rayon::prelude::*;
use crate::piece::move_validators::is_piece_move_valid;
use crate::search::alphabeta::alphabeta;
use crate::search::locking::{get_tt_mutex};
use crate::search::playing_strength::{select_move_based_using_strength, strength_noise_sigma, PLAYING_STRENGTH_MAX};
use crate::search::root_moves::{build_pv_for_root, get_root_moves, hard_root_filter, root_move_bonus};
use crate::search::see::{see_dest_estimate, SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP};
use crate::search::threading::init_rayon_pool_if_needed;
use crate::search::uci_feedback::emit_info;

// ==========================
// Tunable search parameters
// ==========================
// Evaluation bounds used as sentinels inside search
pub const MIN_EVAL_VALUE: i32 = i32::MIN + 100_000;
pub const MAX_EVAL_VALUE: i32 = i32::MAX - 100_000;

pub const DEFAULT_SEARCH_DEPTH: usize = 15;

// Iterative deepening aspiration window (in centipawns)
const ASP_WINDOW_INIT_CP: i32 = 50; // initial aspiration half-window
const ASP_WINDOW_MAX_CP: i32 = 800; // maximum expanded half-window

// Root parallelization thresholds
const ROOT_PARALLEL_MIN_DEPTH: usize = 6; // enable root parallel only from this depth
const ROOT_PARALLEL_MIN_MOVES: usize = 4; // and when at least this many root moves exist

const QUEEN_LOSING_PEN_BASE_CP: i32 = 1000; // base penalty for clearly losing queen moves
const QUEEN_PAWN_ATTACK_EXTRA_CP: i32 = 500; // extra penalty if queen landing square is pawn-attacked

// Root capture adjustment
const ROOT_CAPTURE_BONUS_DIV: i32 = 10; // add captured piece value / this divisor

// Endgame/50-move rule aware root adjustments
const ENDGAME_SIDEADV_THRESHOLD_CP: i32 = 150; // only apply adjustments if side advantage above this
const ENDGAME_HMC_THRESHOLD: u32 = 80; // start scaling after this half-move clock
const ENDGAME_SCALE_MAX: i32 = 21; // max scaling steps used in formula below
const ENDGAME_CAPTURE_SCALE_BONUS_CP: i32 = 15; // per-scale bonus if capture or pawn move
const ENDGAME_NONCAP_SCALE_PENALTY_CP: i32 = 8; // per-scale penalty if quiet move

// History heuristic cap

// Internal stacks/containers sizing
const REP_STACK_CAPACITY: usize = 128; // repetition detection stack capacity hint
// Root repetition-avoidance bias when a move would immediately create 3-fold
const REP_AVOIDANCE_BIAS_CP: i32 = 50_000;



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

    let tt_mutex = get_tt_mutex();

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
    let ps = if playing_strength == 0 {
        1
    } else {
        playing_strength.min(PLAYING_STRENGTH_MAX)
    } as i32;

    let effective_depth = search_depth;

    // Root-level hard 3-fold avoidance: filter out any root move that would create
    // a third occurrence of the same position (per truncated FEN used in History).
    // If filtering removes all moves (e.g., only repetition saves a loss), fall back to all moves.
    let root_moves: Vec<((usize, usize), (usize, usize))> = {
        let mut v = Vec::with_capacity(moves.len());

        get_root_moves(game_state, history, board, active_color, &moves, &mut v);

        // Hard root filter: drop unsafe queen moves (SEE<0) and unsafe minor-piece non-check sacs
        // (SEE<=SEE_MINOR_SAC_THRESHOLD_CP and not giving check)
        // If filtering removes all, keep original set.
        if !v.is_empty() {
            let mut filtered: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(v.len());
            hard_root_filter(board, active_color, &mut v, &mut filtered);
            if filtered.is_empty() { v } else { filtered }
        } else {
            v
        }
    };

    // Iterative Deepening + Aspiration windows at root (serial evaluation for stability)
    // Reuse persistent TT
    let mut tt = tt_mutex.lock().unwrap();
    let base_hmc = game_state.half_move_clock();
    let mut last_score: i32 = 0;
    let mut chosen: Option<((usize, usize), (usize, usize), i32, usize)> = None;
    let mut window: i32 = ASP_WINDOW_INIT_CP; // cp

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
            let mut best_score_raw = if active_color == Color::White {
                MIN_EVAL_VALUE
            } else {
                MAX_EVAL_VALUE
            };
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

            let enable_parallel =
                depth_now >= ROOT_PARALLEL_MIN_DEPTH && ordered.len() >= ROOT_PARALLEL_MIN_MOVES;
            if enable_parallel {
                // 1) Search the first (best-ordered) move serially to establish PV and bounds
                let &(pv_from, pv_to) = ordered.first().unwrap();
                {
                    let mut tmp = board.clone();
                    let u = tmp.make_move_simple(pv_from, pv_to);
                    let moved_is_pawn = board
                        .get(pv_from.0, pv_from.1)
                        .map(|p| p.get_type() == PieceType::Pawn)
                        .unwrap_or(false);
                    let is_capture = board.get(pv_to.0, pv_to.1).is_some();
                    let child_hmc: u32 = if is_capture || moved_is_pawn {
                        0
                    } else {
                        base_hmc.saturating_add(1)
                    };
                    let score_raw = if depth_now <= 1 {
                        evaluate_position(&tmp)
                    } else {
                        let mut rep_stack: Vec<u64> = Vec::with_capacity(REP_STACK_CAPACITY);
                        alphabeta(
                            &mut tmp,
                            opposite_color(active_color),
                            depth_now - 1,
                            a,
                            b,
                            1,
                            &mut tt,
                            child_hmc,
                            &mut rep_stack,
                        )
                    };
                    tmp.unmake_move_simple(u);

                    // Adjust score for root-only heuristics
                    let mut adjusted =
                        score_raw + root_move_bonus(&board, pv_from, pv_to, active_color);
                    // Root SEE gate: penalize moves with negative SEE on destination
                    {
                        let mut post = board.clone();
                        let moved_piece = board.get(pv_from.0, pv_from.1);
                        let captured = board.get(pv_to.0, pv_to.1);
                        if let Some(mp) = moved_piece {
                            post.set(pv_from.0, pv_from.1, None);
                            post.set(pv_to.0, pv_to.1, Some(mp));
                            let cap_val =
                                captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
                            let see = see_dest_estimate(&post, active_color, pv_to, cap_val);
                            if see < 0 {
                                // Hard demotion for losing queen moves at root
                                if mp.get_type() == PieceType::Queen {
                                    let mut pawn_attacked = false;
                                    let opp = opposite_color(active_color);
                                    let (r, c) = (pv_to.0, pv_to.1);
                                    if opp == Color::White {
                                        if r >= 1 {
                                            if c >= 1 {
                                                if let Some(p) = post.get(r - 1, c - 1) {
                                                    if p.get_color() == opp
                                                        && p.get_type() == PieceType::Pawn
                                                    {
                                                        pawn_attacked = true;
                                                    }
                                                }
                                            }
                                            if c + 1 < 8 {
                                                if let Some(p) = post.get(r - 1, c + 1) {
                                                    if p.get_color() == opp
                                                        && p.get_type() == PieceType::Pawn
                                                    {
                                                        pawn_attacked = true;
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        if r + 1 < 8 {
                                            if c >= 1 {
                                                if let Some(p) = post.get(r + 1, c - 1) {
                                                    if p.get_color() == opp
                                                        && p.get_type() == PieceType::Pawn
                                                    {
                                                        pawn_attacked = true;
                                                    }
                                                }
                                            }
                                            if c + 1 < 8 {
                                                if let Some(p) = post.get(r + 1, c + 1) {
                                                    if p.get_color() == opp
                                                        && p.get_type() == PieceType::Pawn
                                                    {
                                                        pawn_attacked = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let mut pen = (-see).max(QUEEN_LOSING_PEN_BASE_CP); // decisive ban at root
                                    if pawn_attacked {
                                        pen += QUEEN_PAWN_ATTACK_EXTRA_CP;
                                    }
                                    adjusted -= pen;
                                } else {
                                    let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP);
                                    adjusted -= pen;
                                }
                            }
                        }
                    }
                    if let Some(captured) = board.get(pv_to.0, pv_to.1) {
                        let cap_val = capture_value_cp(captured.get_type());
                        adjusted += cap_val / ROOT_CAPTURE_BONUS_DIV;
                    }
                    let side_adv = if active_color == Color::White {
                        score_raw
                    } else {
                        -score_raw
                    };
                    if side_adv > ENDGAME_SIDEADV_THRESHOLD_CP && base_hmc >= ENDGAME_HMC_THRESHOLD
                    {
                        if is_capture || moved_is_pawn {
                            let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1))
                                .min(ENDGAME_SCALE_MAX);
                            adjusted += ENDGAME_CAPTURE_SCALE_BONUS_CP * scale;
                        } else {
                            let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1))
                                .min(ENDGAME_SCALE_MAX);
                            adjusted -= ENDGAME_NONCAP_SCALE_PENALTY_CP * scale;
                        }
                    }
                    let sigma = strength_noise_sigma(ps as usize);
                    if sigma > 0 {
                        let n: i32 = rng().random_range(-sigma..=sigma);
                        adjusted += n;
                    }

                    best_from_to = Some((pv_from, pv_to));
                    best_adjusted = adjusted;
                    best_score_raw = score_raw;
                }

                // 2) Search the remaining moves in parallel with per-task local TT to avoid contention
                let base_board = board.clone();
                let base_hmc_loc = base_hmc;
                let a_loc = a;
                let b_loc = b;
                let side = active_color;
                let results = ordered[1..]
                    .par_iter()
                    .map(|&(from, to)| {
                        let mut tmp = base_board.clone();
                        let u = tmp.make_move_simple(from, to);
                        let moved_is_pawn = base_board
                            .get(from.0, from.1)
                            .map(|p| p.get_type() == PieceType::Pawn)
                            .unwrap_or(false);
                        let is_capture = base_board.get(to.0, to.1).is_some();
                        let child_hmc: u32 = if is_capture || moved_is_pawn {
                            0
                        } else {
                            base_hmc_loc.saturating_add(1)
                        };
                        let score_raw = if depth_now <= 1 {
                            evaluate_position(&tmp)
                        } else {
                            // local TT per task
                            let mut local_tt = TranspositionTable::new_with_default_size();
                            let mut rep_stack: Vec<u64> = Vec::with_capacity(REP_STACK_CAPACITY);
                            alphabeta(
                                &mut tmp,
                                opposite_color(side),
                                depth_now - 1,
                                a_loc,
                                b_loc,
                                1,
                                &mut local_tt,
                                child_hmc,
                                &mut rep_stack,
                            )
                        };
                        tmp.unmake_move_simple(u);

                        // Root adjustments (skip repetition-history check to keep parallel code simple)
                        let mut adjusted = score_raw + root_move_bonus(&base_board, from, to, side);
                        // Root SEE gate: penalize moves with negative SEE on destination
                        {
                            let mut post = base_board.clone();
                            let moved_piece = base_board.get(from.0, from.1);
                            let captured = base_board.get(to.0, to.1);
                            if let Some(mp) = moved_piece {
                                post.set(from.0, from.1, None);
                                post.set(to.0, to.1, Some(mp));
                                let cap_val =
                                    captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
                                let see = see_dest_estimate(&post, side, to, cap_val);
                                if see < 0 {
                                    if mp.get_type() == PieceType::Queen {
                                        let mut pawn_attacked = false;
                                        let opp = opposite_color(side);
                                        let (r, c) = (to.0, to.1);
                                        if opp == Color::White {
                                            if r >= 1 {
                                                if c >= 1 {
                                                    if let Some(p) = post.get(r - 1, c - 1) {
                                                        if p.get_color() == opp
                                                            && p.get_type() == PieceType::Pawn
                                                        {
                                                            pawn_attacked = true;
                                                        }
                                                    }
                                                }
                                                if c + 1 < 8 {
                                                    if let Some(p) = post.get(r - 1, c + 1) {
                                                        if p.get_color() == opp
                                                            && p.get_type() == PieceType::Pawn
                                                        {
                                                            pawn_attacked = true;
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            if r + 1 < 8 {
                                                if c >= 1 {
                                                    if let Some(p) = post.get(r + 1, c - 1) {
                                                        if p.get_color() == opp
                                                            && p.get_type() == PieceType::Pawn
                                                        {
                                                            pawn_attacked = true;
                                                        }
                                                    }
                                                }
                                                if c + 1 < 8 {
                                                    if let Some(p) = post.get(r + 1, c + 1) {
                                                        if p.get_color() == opp
                                                            && p.get_type() == PieceType::Pawn
                                                        {
                                                            pawn_attacked = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let mut pen = (-see).max(1000);
                                        if pawn_attacked {
                                            pen += 500;
                                        }
                                        adjusted -= pen;
                                    } else {
                                        let pen = (-see).clamp(80, 300);
                                        adjusted -= pen;
                                    }
                                }
                            }
                        }
                        if let Some(captured) = base_board.get(to.0, to.1) {
                            let cap_val = capture_value_cp(captured.get_type());
                            adjusted += cap_val / 10;
                        }
                        let side_adv = if side == Color::White {
                            score_raw
                        } else {
                            -score_raw
                        };
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
                        if sigma > 0 {
                            let n: i32 = rng().random_range(-sigma..=sigma);
                            adjusted += n;
                        }
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
                for &(from, to) in &ordered {
                    let mut tmp = board.clone();
                    let u = tmp.make_move_simple(from, to);
                    let moved_is_pawn = board
                        .get(from.0, from.1)
                        .map(|p| p.get_type() == PieceType::Pawn)
                        .unwrap_or(false);
                    let is_capture = board.get(to.0, to.1).is_some();
                    let child_hmc: u32 = if is_capture || moved_is_pawn {
                        0
                    } else {
                        base_hmc.saturating_add(1)
                    };
                    let score_raw = if depth_now <= 1 {
                        evaluate_position(&tmp)
                    } else {
                        let mut rep_stack: Vec<u64> = Vec::with_capacity(REP_STACK_CAPACITY);
                        alphabeta(
                            &mut tmp,
                            opposite_color(active_color),
                            depth_now - 1,
                            a,
                            b,
                            1,
                            &mut tt,
                            child_hmc,
                            &mut rep_stack,
                        )
                    };
                    tmp.unmake_move_simple(u);

                    // Adjust score for root-only heuristics
                    let mut adjusted = score_raw + root_move_bonus(&board, from, to, active_color);
                    if let Some(captured) = board.get(to.0, to.1) {
                        let cap_val = capture_value_cp(captured.get_type());
                        adjusted += cap_val / ROOT_CAPTURE_BONUS_DIV;
                    }
                    let side_adv = if active_color == Color::White {
                        score_raw
                    } else {
                        -score_raw
                    };
                    if side_adv > ENDGAME_SIDEADV_THRESHOLD_CP && base_hmc >= ENDGAME_HMC_THRESHOLD
                    {
                        if is_capture || moved_is_pawn {
                            let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1))
                                .min(ENDGAME_SCALE_MAX);
                            adjusted += ENDGAME_CAPTURE_SCALE_BONUS_CP * scale;
                        } else {
                            let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1))
                                .min(ENDGAME_SCALE_MAX);
                            adjusted -= ENDGAME_NONCAP_SCALE_PENALTY_CP * scale;
                        }
                    }
                    let sigma = strength_noise_sigma(ps as usize);
                    if sigma > 0 {
                        let n: i32 = rng().random_range(-sigma..=sigma);
                        adjusted += n;
                    }
                    // repetition-avoidance at root
                    {
                        let is_capture = board.get(to.0, to.1).is_some();
                        let mut gs = game_state; // Copy
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
                            let truncated =
                                fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
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
                    }

                    // Track best
                    let better = if active_color == Color::White {
                        adjusted > best_adjusted
                    } else {
                        adjusted < best_adjusted
                    };
                    if better || best_from_to.is_none() {
                        best_from_to = Some((from, to));
                        best_adjusted = adjusted;
                        best_score_raw = score_raw;
                    }
                    // Aspiration cutoffs help ordering mid-loop too
                    if active_color == Color::White && score_raw >= b {
                        break;
                    }
                    if active_color == Color::Black && score_raw <= a {
                        break;
                    }
                }
            }

            // Check aspiration result
            if best_score_raw <= a {
                // fail-low: widen down
                window = (window * 2).min(ASP_WINDOW_MAX_CP);
                a = (last_score - window).max(MIN_EVAL_VALUE + 1);
                if tried < 3 {
                    continue;
                }
            } else if best_score_raw >= b {
                // fail-high: widen up
                window = (window * 2).min(ASP_WINDOW_MAX_CP);
                b = (last_score + window).min(MAX_EVAL_VALUE - 1);
                if tried < 3 {
                    continue;
                }
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
                let u = tmp.make_move_simple(from, to);
                let moved_is_pawn = board
                    .get(from.0, from.1)
                    .map(|p| p.get_type() == PieceType::Pawn)
                    .unwrap_or(false);
                let is_capture = board.get(to.0, to.1).is_some();
                let child_hmc: u32 = if is_capture || moved_is_pawn {
                    0
                } else {
                    base_hmc.saturating_add(1)
                };
                let mut rep_stack: Vec<u64> = Vec::with_capacity(64);
                let sr = if effective_depth <= 1 {
                    evaluate_position(&tmp)
                } else {
                    alphabeta(
                        &mut tmp,
                        opposite_color(active_color),
                        effective_depth - 1,
                        MIN_EVAL_VALUE + 1,
                        MAX_EVAL_VALUE - 1,
                        1,
                        &mut tt,
                        child_hmc,
                        &mut rep_stack,
                    )
                };
                tmp.unmake_move_simple(u);
                // Apply root_move_bonus plus SEE-based penalty same as earlier paths
                let mut adj = sr + root_move_bonus(&board, from, to, active_color);
                {
                    let mut post = board.clone();
                    let moved_piece = board.get(from.0, from.1);
                    let captured = board.get(to.0, to.1);
                    if let Some(mp) = moved_piece {
                        post.set(from.0, from.1, None);
                        post.set(to.0, to.1, Some(mp));
                        let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
                        let see = see_dest_estimate(&post, active_color, to, cap_val);
                        if see < 0 {
                            if mp.get_type() == PieceType::Queen {
                                // extra demotion for losing queen moves at root
                                let mut pawn_attacked = false;
                                let opp = opposite_color(active_color);
                                let (r, c) = (to.0, to.1);
                                if opp == Color::White {
                                    if r >= 1 {
                                        if c >= 1 {
                                            if let Some(p) = post.get(r - 1, c - 1) {
                                                if p.get_color() == opp
                                                    && p.get_type() == PieceType::Pawn
                                                {
                                                    pawn_attacked = true;
                                                }
                                            }
                                        }
                                        if c + 1 < 8 {
                                            if let Some(p) = post.get(r - 1, c + 1) {
                                                if p.get_color() == opp
                                                    && p.get_type() == PieceType::Pawn
                                                {
                                                    pawn_attacked = true;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    if r + 1 < 8 {
                                        if c >= 1 {
                                            if let Some(p) = post.get(r + 1, c - 1) {
                                                if p.get_color() == opp
                                                    && p.get_type() == PieceType::Pawn
                                                {
                                                    pawn_attacked = true;
                                                }
                                            }
                                        }
                                        if c + 1 < 8 {
                                            if let Some(p) = post.get(r + 1, c + 1) {
                                                if p.get_color() == opp
                                                    && p.get_type() == PieceType::Pawn
                                                {
                                                    pawn_attacked = true;
                                                }
                                            }
                                        }
                                    }
                                }
                                let mut pen = (-see).max(1000);
                                if pawn_attacked {
                                    pen += 500;
                                }
                                adj -= pen;
                            } else {
                                let pen = (-see).clamp(80, 300);
                                adj -= pen;
                            }
                        }
                    }
                }
                scored.push((from, to, adj));
            }

            let mut sorted = sort_moves_on_score_asc(&mut scored);
            if active_color == Color::White {
                sorted.reverse();
            }

            if let Some((from, to)) = select_move_based_using_strength(&sorted, playing_strength) {
                let sc = sorted
                    .iter()
                    .find(|e| e.0 == from && e.1 == to)
                    .map(|e| e.2)
                    .unwrap_or(sc);
                Some((from, to, sc, used_depth))
            } else {
                Some((bf, bt, sc, used_depth))
            }
        }
    } else {
        None
    }
}

pub(crate) fn find_all_valid_moves(
    board: &Board,
    active_color: Color,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut result: Vec<((usize, usize), (usize, usize))> = Vec::new();

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
                    let is_capture = target_piece_is_some;
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !board.move_from_and_to_validation_check(
                        from,
                        to,
                        active_color,
                        is_capture,
                        is_pawn_move,
                        None,
                    ) {
                        continue;
                    }

                    if is_piece_move_valid(
                        board,
                        active_color,
                        r,
                        c,
                        piece,
                        tr,
                        tc,
                        from,
                        to,
                        is_capture,
                    ) {
                        result.push((from, to));
                    }
                }
            }
        }
    }
    result
}

// Sorts the move table by score in ascending order and returns a cloned, sorted vector.
fn sort_moves_on_score_asc(
    move_table: &mut Vec<((usize, usize), (usize, usize), i32)>,
) -> Vec<((usize, usize), (usize, usize), i32)> {
    move_table.sort_by_key(|m| m.2);
    move_table.clone()
}
