use crate::history::history::History;
use crate::piece::pieces::{opposite_color, Color, Piece, PieceType};
use crate::search::management::aspiration::{probe_with_aspiration, ASP_WINDOW_INIT_CP};
use crate::search::state::locking::get_tt_mutex;
use crate::search::integration::playing_strength::select_move_based_using_strength_promo;

// Re-export find_all_valid_moves for backward compatibility
pub use crate::search::management::move_generator::find_all_valid_moves;
use crate::search::evaluation::root_evaluator::evaluate_root_for_bounds;
use crate::search::management::root_moves::{
    adjusted_root_eval_for_move, build_pv_for_root, get_root_moves,
};
use crate::search::integration::threading::init_rayon_pool_if_needed;
use crate::search::integration::uci_feedback::emit_info;
use crate::state::game_state::GameState;
pub(crate) use crate::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};

pub const SEARCH_ABORTED: i32 = MAX_EVAL_VALUE + 50000;
pub(crate) use crate::book::book::{book_pick, get_order_book_enabled};
use crate::search::Search;
use crate::board::san_move::convert_move_to_san;
use crate::search::core::alphabeta::with_heuristics;

pub const DEFAULT_SEARCH_DEPTH: usize = 15;
pub const MAX_SEARCH_DEPTH: usize = 20;

// Global toggle to enable/disable Zobrist hashing across the engine.
// When disabled, features relying on Zobrist keys (like TT and repetition checks)
// will be bypassed.
pub(crate) const ZOBRIST_HASHING_ENABLED: bool = true;

// Tie the transposition table to Zobrist hashing. Without Zobrist keys, TT is disabled.
pub(crate) const TRANSPOSITION_TABLE_ENABLED: bool = ZOBRIST_HASHING_ENABLED;

pub(crate) const NULL_MOVE_PRUNING_ENABLED: bool = true;
pub(crate) const ASPIRATION_WINDOWS_ENABLED: bool = true;

pub(crate) const QUIESCENCE_ENABLED: bool = true;
pub(crate) const QSEE_PRUNING_ENABLED: bool = true;
pub(crate) const MVV_LVA_ENABLED: bool = true;
pub(crate) const LMR_ENABLED: bool = true;
pub(crate) const ID_ITERATIONS_ENABLED: bool = true;

pub const MAX_PLAYING_STRENGTH: usize = 1000;
pub const DEFAULT_MOVE_TIME_FOR_STRENGTH_MODE_PLAY: usize = 3000usize;

// Re-export move generator types for backward compatibility
pub use crate::search::management::move_generator::{find_all_valid_moves_into_perft, PerftMove, _dump_all_valid_moves};

// Provide a simple implementor of the `Search` trait that forwards to this module's function.
// This keeps existing callers of the free function intact while enabling trait-based use.
pub struct AdvancedSearch;

impl Search for AdvancedSearch {
    fn find_best_move(
        game_state: &GameState,
        history: &History,
        search_depth: usize,
        playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
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
) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
    find_best_move_internal(game_state, history, search_depth, playing_strength, None)
}

/// Find the best move and return all root moves ranked by score.
/// Returns a vector of (SAN, adjusted_score, raw_score) sorted by preference for the side to move.
/// Uses full iterative deepening and all search optimizations, making it much faster than debug_rank_root_moves.
pub fn find_best_move_with_ranking(
    game_state: &GameState,
    history: &History,
    search_depth: usize,
) -> Vec<(String, i32, i32)> {
    let mut all_scores = Vec::new();
    find_best_move_internal(game_state, history, search_depth, MAX_PLAYING_STRENGTH, Some(&mut all_scores));

    // Convert moves to SAN notation
    let active_color = game_state.active_color();
    let mut result: Vec<(String, i32, i32)> = all_scores
        .into_iter()
        .map(|((from, to, promo), adj, raw)| {
            let san = convert_move_to_san(*game_state, Some((from, to, promo))).unwrap_or_else(|| {
                format!("{}{}", crate::piece::as_square_str(from), crate::piece::as_square_str(to))
            });
            (san, adj, raw)
        })
        .collect();

    // Sort by adjusted score (descending for White, ascending for Black)
    if active_color == Color::White {
        result.sort_by(|a, b| b.1.cmp(&a.1));
    } else {
        result.sort_by(|a, b| a.1.cmp(&b.1));
    }

    result
}

/// Internal implementation that optionally collects all move rankings
fn find_best_move_internal(
    game_state: &GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
    mut all_move_scores: Option<&mut Vec<(((usize, usize), (usize, usize), Option<char>), i32, i32)>>,
) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
    init_rayon_pool_if_needed();

    // Clear heuristics for deterministic search
    with_heuristics(|h| h.clear());

    let mut gs = *game_state;
    let tt_mutex = get_tt_mutex();
    {
        let mut tt_lock = tt_mutex.lock().unwrap();
        if crate::search::is_deterministic() {
            tt_lock.clear();
        }
    }

    // Unified move loop - works for both maximizing (White) and minimizing (Black)
    let gen_moves = find_all_valid_moves(&mut gs);
    let active_color = gs.active_color();

    if gen_moves.is_empty() {
        return None;
    }

    // Opening book: if we have a book move in early game, play it immediately.
    if get_order_book_enabled()
        && gs.full_move_number() <= 8
            && let Some((bf, bt)) = book_pick(&gs) {
                return Some((bf, bt, None, 0, 0));
            }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if search_depth == 0 { 1 } else { search_depth };

    let effective_depth = search_depth;

    let mut root_moves: Vec<((usize, usize), (usize, usize), Option<char>)> = {
        let mut v = Vec::with_capacity(gen_moves.len());

        get_root_moves(&mut gs, history, active_color, &gen_moves, &mut v);

        let mut base = v;

        // Heuristic ordering at root: prioritize checking moves, then captures by MVV, then others.
        base.sort_by(|&(f1,t1,p1_promo), &(f2,t2,p2_promo)| {
            use crate::piece::pieces::piece_value_cp;
            let mut b1 = *gs.board();
            let mut b2 = *gs.board();
            let p1 = gs.board().get(f1.0, f1.1);
            let p2 = gs.board().get(f2.0, f2.1);
            let cap1 = gs.board().get(t1.0, t1.1);
            let cap2 = gs.board().get(t2.0, t2.1);
            if let Some(mut mp1) = p1 {
                if mp1.get_type() == PieceType::Pawn && p1_promo.is_some() {
                    mp1 = Piece::new(PieceType::Queen, mp1.get_color());
                }
                b1.set(t1.0, t1.1, Some(mp1));
                b1.set(f1.0, f1.1, None);
                if mp1.get_type() == PieceType::King {
                    b1.set_king_location(mp1.get_color(), t1);
                }
            }
            if let Some(mut mp2) = p2 {
                if mp2.get_type() == PieceType::Pawn && p2_promo.is_some() {
                    mp2 = Piece::new(PieceType::Queen, mp2.get_color());
                }
                b2.set(t2.0, t2.1, Some(mp2));
                b2.set(f2.0, f2.1, None);
                if mp2.get_type() == PieceType::King {
                    b2.set_king_location(mp2.get_color(), t2);
                }
            }
            let check1 = if p1.is_some() { b1.is_side_in_check(opposite_color(active_color)) } else { false };
            let check2 = if p2.is_some() { b2.is_side_in_check(opposite_color(active_color)) } else { false };

            let mut key1 = (check1 as i32) * 10000 + cap1.map(|pc| piece_value_cp(pc.get_type())).unwrap_or(0);
            let mut key2 = (check2 as i32) * 10000 + cap2.map(|pc| piece_value_cp(pc.get_type())).unwrap_or(0);

            if let Some(_promo) = p1_promo { key1 += 900; }
            if let Some(_promo) = p2_promo { key2 += 900; }

            key2.cmp(&key1) // descending
        });
        base
    };

    // Iterative Deepening + Aspiration windows at root (serial evaluation for stability)
    let mut tt = tt_mutex.lock().unwrap();
    let mut _last_score: i32 = 0;
    let mut chosen: Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> = None;

    if ID_ITERATIONS_ENABLED {
        for depth_now in 1..=effective_depth {
            tt.next_age();
            // Reset aspiration window at the start of each iteration
            let mut window: i32 = ASP_WINDOW_INIT_CP;
            // Only collect scores on final depth iteration
            let should_collect = depth_now == effective_depth && all_move_scores.is_some();
            let ((bf, bt, bpromo), best_adj, best_raw) = if ASPIRATION_WINDOWS_ENABLED {
                probe_with_aspiration(
                    active_color,
                    &root_moves,
                    depth_now,
                    _last_score,
                    &mut window,
                    &mut tt,
                    &mut gs,
                    history,
                    if should_collect { all_move_scores.as_deref_mut() } else { None },
                )
            } else {
                evaluate_root_for_bounds(
                    active_color,
                    &root_moves,
                    depth_now,
                    MIN_EVAL_VALUE + 1,
                    MAX_EVAL_VALUE - 1,
                    &mut tt,
                    &mut gs,
                    history,
                    if should_collect { all_move_scores.as_deref_mut() } else { None },
                )
            };

            if best_raw == SEARCH_ABORTED {
                if chosen.is_none() {
                    let (bf, bt, bpromo) = root_moves[0];
                    chosen = Some((bf, bt, bpromo, 0, 1));
                }
                break;
            }

            _last_score = best_raw;
            // Emit PV/info for this iteration, including TT hashfull permille
            let hf = tt.hashfull_permille();
            let pv = build_pv_for_root(&gs, bf, bt, bpromo, &tt, depth_now);
            let white_persp_score = if active_color == Color::Black { -best_adj } else { best_adj };
            emit_info(bf, bt, bpromo, white_persp_score, depth_now, pv, hf);
            chosen = Some((bf, bt, bpromo, best_adj, depth_now));

            // Reorder root moves: place best move from this iteration first for next iteration
            if let Some(pos) = root_moves.iter().position(|&(f, t, p)| f == bf && t == bt && p == bpromo) {
                if pos > 0 {
                    let best = root_moves.remove(pos);
                    root_moves.insert(0, best);
                }
            }
        }
    } else {
        // Single-depth Search without iterative deepening
        let depth_now = effective_depth;
        tt.next_age();
        let mut window: i32 = ASP_WINDOW_INIT_CP;
        let ((bf, bt, bpromo), best_adj, best_raw) = if ASPIRATION_WINDOWS_ENABLED {
            probe_with_aspiration(
                active_color,
                &root_moves,
                depth_now,
                _last_score,
                &mut window,
                &mut tt,
                &mut gs,
                history,
                all_move_scores.as_deref_mut(),
            )
        } else {
            evaluate_root_for_bounds(
                active_color,
                &root_moves,
                depth_now,
                MIN_EVAL_VALUE + 1,
                MAX_EVAL_VALUE - 1,
                &mut tt,
                &mut gs,
                history,
                all_move_scores.as_deref_mut(),
            )
        };

        if best_raw == SEARCH_ABORTED {
            let (bf, bt, bpromo) = root_moves[0];
            chosen = Some((bf, bt, bpromo, 0, 1));
        } else {
            let hf = tt.hashfull_permille();
            let pv = build_pv_for_root(&gs, bf, bt, bpromo, &tt, depth_now);
            let white_persp_score = if active_color == Color::Black { -best_adj } else { best_adj };
            emit_info(bf, bt, bpromo, white_persp_score, depth_now, pv, hf);
            chosen = Some((bf, bt, bpromo, best_adj, depth_now));
        }
    }

    // Final selection based on playing_strength from the last iteration
    // Use TT-cached scores from iterative deepening to avoid expensive re-evaluation
    if let Some((_bf, _bt, _bpromo, sc, used_depth)) = chosen
        && playing_strength < MAX_PLAYING_STRENGTH {
            // Collect scores from TT for all root moves (already evaluated during iterative deepening)
            let mut scored_with_promo: Vec<((usize, usize), (usize, usize), Option<char>, i32)> = Vec::new();

            // Apply evaluation noise if not in deterministic mode
            use crate::search::integration::playing_strength::strength_noise_sigma;
            use rand::{rng, Rng};
            let apply_noise = !crate::search::is_deterministic() && playing_strength < 1000;
            let sigma = if apply_noise { strength_noise_sigma(playing_strength) } else { 0 };

            for &(from, to, promo) in &root_moves {
                // Try to get score from TT instead of re-evaluating
                use crate::search::state::zobrist::compute_zobrist_full;
                use crate::piece::piece_mover::PieceMover;

                let mut temp_gs = gs;
                let is_capture = temp_gs.board().get(to.0, to.1).is_some();
                let moved_piece = temp_gs.board().get(from.0, from.1);
                let moved_is_pawn = moved_piece.map(|p| p.get_type() == PieceType::Pawn).unwrap_or(false);

                let promotion_piece = if let Some(promo_char) = promo {
                    Piece::from_fen_char(promo_char)
                } else {
                    None
                };

                if PieceMover::move_piece(&mut temp_gs, from, to, is_capture, promotion_piece) {
                    temp_gs.switch_player_turn();
                    let key = compute_zobrist_full(
                        temp_gs.board(),
                        temp_gs.active_color(),
                        &temp_gs.castling_rights(),
                        temp_gs.en_passant_target(),
                    );

                    // Look up in TT; if not found, use a heuristic estimate
                    let score_raw = if let Some(entry) = tt.probe(key) {
                        use crate::search::state::tt::from_tt_score;
                        -from_tt_score(entry.score, 0) // Negate because it's from opponent's perspective
                    } else {
                        // Fallback: simple static evaluation if not in TT
                        use crate::board::evaluator::evaluate_position;
                        let eval = evaluate_position(temp_gs.board(), active_color);
                        if active_color == Color::Black { -eval } else { eval }
                    };

                    let mut adj = adjusted_root_eval_for_move(
                        gs.board(),
                        active_color,
                        from,
                        to,
                        gs.half_move_clock(),
                        score_raw,
                        is_capture,
                        moved_is_pawn,
                    );

                    // Apply evaluation noise to introduce positional misjudgment at lower strengths
                    if apply_noise && sigma > 0 {
                        // Box-Muller transform for Gaussian noise
                        let u1: f32 = rng().random::<f32>();
                        let u2: f32 = rng().random::<f32>();
                        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
                        let noise = (z0 * sigma as f32) as i32;
                        adj += noise;
                    }

                    scored_with_promo.push((from, to, promo, adj));
                }
            }

            if !scored_with_promo.is_empty() {
                scored_with_promo.sort_by_key(|m| m.3);
                if active_color == Color::White {
                    scored_with_promo.reverse();
                }

                if let Some((from, to, promo_opt)) = select_move_based_using_strength_promo(&scored_with_promo, playing_strength) {
                    let sc_final = scored_with_promo
                        .iter()
                        .find(|e| e.0 == from && e.1 == to && e.2 == promo_opt)
                        .map(|e| e.3)
                        .unwrap_or(sc);
                    chosen = Some((from, to, promo_opt, sc_final, used_depth));
                }
            }
        }

    if let Some((bf, bt, _bpromo, sc, used_depth)) = chosen {
        return Some((bf, bt, _bpromo, sc, used_depth));
    }
    None
}

/// Debug helper: rank root moves for the current position, returning (SAN, adjusted_score, raw_score)
/// Sorted by side to move preference (White: descending; Black: ascending).
///
/// This function now uses the optimized production search with iterative deepening and all
/// search enhancements, making it much faster than the previous implementation.
/// The depth > 5 restriction has been removed.
pub fn debug_rank_root_moves(
    game_state: &GameState,
    history: &History,
    depth: usize,
) -> Vec<(String, i32, i32)> {
    find_best_move_with_ranking(game_state, history, depth)
}
