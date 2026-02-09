#![doc(hidden)]

use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::search::context::SearchContext;
use crate::search::state::tt::TranspositionTable;
use crate::state::game_state::GameState;

pub use crate::search::evaluation::heuristics::SearchHeuristics;
pub use crate::search::evaluation::root_heuristics::utils::simulate_move;
pub use crate::search::state::rep_stack::RepetitionStack;
pub use crate::search::state::tt::{Bound, from_tt_score, to_tt_score, MATE_TB};
use crate::state::castling::CastlingRights;

pub fn score_raw_for_strength_move(
    temp_gs: &GameState,
    active_color: Color,
    tt: &TranspositionTable,
) -> i32 {
    crate::search::core::advanced_search::score_raw_for_strength_move(temp_gs, active_color, tt)
}

pub fn qsearch(
    ctx: &SearchContext,
    game_state: &mut GameState,
    alpha: i32,
    beta: i32,
    rep_stack: &mut RepetitionStack,
) -> i32 {
    crate::search::core::qsearch::qsearch(ctx, game_state, alpha, beta, rep_stack)
}

pub fn qsearch_with_quiescence(
    ctx: &SearchContext,
    game_state: &mut GameState,
    alpha: i32,
    beta: i32,
    rep_stack: &mut RepetitionStack,
    quiescence_enabled: bool,
) -> i32 {
    crate::search::core::qsearch::qsearch_with_quiescence(
        ctx,
        game_state,
        alpha,
        beta,
        rep_stack,
        quiescence_enabled,
    )
}

pub fn has_search_aborted(results: &[((usize, usize), (usize, usize), Option<char>, i32, i32)]) -> bool {
    crate::search::evaluation::root_evaluator::has_search_aborted(results)
}

pub fn set_test_sleep_after_pv_ms(ms: u64) {
    crate::search::evaluation::root_evaluator::set_test_sleep_after_pv_ms(ms);
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_root_for_bounds(
    ctx: &SearchContext,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &TranspositionTable,
    game_state: &mut GameState,
    history: &History,
    heuristics: &mut SearchHeuristics,
    collect_all_scores: Option<&mut Vec<(((usize, usize), (usize, usize), Option<char>), i32, i32)>>,
) -> (((usize, usize), (usize, usize), Option<char>), i32, i32) {
    crate::search::evaluation::root_evaluator::evaluate_root_for_bounds(
        ctx,
        active_color,
        root_moves,
        depth_now,
        a,
        b,
        tt,
        game_state,
        history,
        heuristics,
        collect_all_scores,
    )
}

pub fn find_all_capture_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    crate::search::management::move_generator::find_all_capture_moves(game_state)
}

pub fn find_all_evasion_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    crate::search::management::move_generator::find_all_evasion_moves(game_state)
}

pub fn see_dest_estimate(
    board_after: &Board,
    side_just_moved: Color,
    dest: (usize, usize),
    captured_val: i32,
) -> i32 {
    crate::search::management::see::see_dest_estimate(board_after, side_just_moved, dest, captured_val)
}

pub fn see_after(board: &Board, side: Color, to: (usize, usize), captured: Option<Piece>) -> i32 {
    crate::search::management::see::see_after(board, side, to, captured)
}

#[allow(clippy::too_many_arguments)]
pub fn calculate_lmr_reduction(
    child_depth: usize,
    move_index: i32,
    is_quiet: bool,
    gives_check: bool,
    allow_reduce: bool,
    to_move: Color,
    from: (usize, usize),
    to: (usize, usize),
    moved_pt: Option<PieceType>,
    board: &Board,
    phase: i32,
    captured: Option<Piece>,
    heuristics: &SearchHeuristics,
    ply: i32,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> usize {
    crate::search::core::alphabeta::calculate_lmr_reduction(
        child_depth,
        move_index,
        is_quiet,
        gives_check,
        allow_reduce,
        to_move,
        from,
        to,
        moved_pt,
        board,
        phase,
        captured,
        heuristics,
        ply,
        prev_move,
    )
}

pub fn attacked_by_pawn(board: &Board, sq: (usize, usize), attacker: Color) -> bool {
    crate::search::management::see::attacked_by_pawn(board, sq, attacker)
}

pub fn pawn_attacked_minor_penalty(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
    moved_pt: PieceType,
) -> i32 {
    crate::search::management::see::pawn_attacked_minor_penalty(post_after, side, to, moved_pt)
}

pub fn aspiration_bounds_for_depth(depth_now: usize, last_score: i32, window: i32, in_check: bool) -> (i32, i32) {
    crate::search::management::aspiration::aspiration_bounds_for_depth(depth_now, last_score, window, in_check)
}

pub fn aspiration_window_init() -> i32 {
    crate::search::management::aspiration::ASP_WINDOW_INIT_CP
}

pub fn aspiration_window_max() -> i32 {
    crate::search::management::aspiration::ASP_WINDOW_MAX_CP
}

pub fn next_aspiration_window(prev_window: i32, score_delta: i32) -> i32 {
    crate::search::management::aspiration::next_aspiration_window(prev_window, score_delta)
}

pub fn should_verify_aspiration(depth_now: usize, window: i32, best_raw: i32) -> bool {
    crate::search::management::aspiration::should_verify_aspiration(depth_now, window, best_raw)
}

#[allow(clippy::too_many_arguments)]
pub fn probe_with_aspiration(
    ctx: &SearchContext,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    depth_now: usize,
    last_score: i32,
    window: &mut i32,
    tt: &TranspositionTable,
    game_state: &mut GameState,
    history: &History,
    heuristics: &mut SearchHeuristics,
    collect_all_scores: Option<&mut Vec<(((usize, usize), (usize, usize), Option<char>), i32, i32)>>,
) -> (((usize, usize), (usize, usize), Option<char>), i32, i32) {
    crate::search::management::aspiration::probe_with_aspiration(
        ctx,
        active_color,
        root_moves,
        depth_now,
        last_score,
        window,
        tt,
        game_state,
        history,
        heuristics,
        collect_all_scores,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn prune_null_moves(
    ctx: &SearchContext,
    heuristics: &mut SearchHeuristics,
    game_state: &mut GameState,
    depth: usize,
    alpha: i32,
    beta: i32,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    allow_null_move: bool,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> Option<i32> {
    crate::search::management::prune_null_moves::prune_null_moves(
        ctx,
        heuristics,
        game_state,
        depth,
        alpha,
        beta,
        ply,
        tt,
        rep_stack,
        allow_null_move,
        prev_move,
    )
}

pub fn compute_zobrist_full(
    board: &Board,
    to_move: Color,
    castling: &CastlingRights,
    ep_target: Option<(usize, usize)>,
) -> u64 {
    crate::search::state::zobrist::compute_zobrist_full(board, to_move, castling, ep_target)
}

pub fn zobrist_update_ep(
    key: u64,
    old_ep: Option<(usize, usize)>,
    new_ep: Option<(usize, usize)>,
) -> u64 {
    crate::search::state::zobrist::zobrist_update_ep(key, old_ep, new_ep)
}

#[allow(clippy::too_many_arguments)]
pub fn apply_repetition_avoidance_bias(
    adjusted: i32,
    game_state: &GameState,
    history: &History,
    active_color: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score_raw: i32,
) -> i32 {
    crate::search::evaluation::repetition::apply_repetition_avoidance_bias(
        adjusted,
        game_state,
        history,
        active_color,
        from,
        to,
        promo,
        score_raw,
    )
}

pub fn check_mobility_bonus_for_side(post_after: &Board, checked_side: Color) -> i32 {
    crate::search::evaluation::root_heuristics::check_hanging::check_mobility_bonus_for_side(
        post_after,
        checked_side,
    )
}

pub fn critical_square_defense_bonus(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    crate::search::evaluation::root_heuristics::critical_defense::critical_square_defense_bonus(
        base_board,
        post_after,
        side,
        from,
        to,
    )
}

pub fn endgame_50move_scaling(
    side: Color,
    score_raw: i32,
    base_hmc: u32,
    is_capture: bool,
    moved_is_pawn: bool,
) -> i32 {
    crate::search::evaluation::root_heuristics::endgame_scaling::endgame_50move_scaling(
        side,
        score_raw,
        base_hmc,
        is_capture,
        moved_is_pawn,
    )
}

pub fn king_safety_root_heuristics(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
) -> i32 {
    crate::search::evaluation::root_heuristics::king_safety::king_safety_root_heuristics(
        base_board,
        side,
        from,
        to,
        is_capture,
    )
}

pub fn knight_safe_squares(board: &Board, side: Color, from: (usize, usize)) -> Vec<(usize, usize)> {
    crate::search::evaluation::root_heuristics::knight_evacuation::knight_safe_squares(
        board,
        side,
        from,
    )
}

pub fn knight_evacuations_priority(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    gives_check: bool,
) -> i32 {
    crate::search::evaluation::root_heuristics::knight_evacuation::knight_evacuations_priority(
        base_board,
        side,
        from,
        to,
        gives_check,
    )
}

pub fn opponent_knight_check_fork_penalty(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
) -> i32 {
    crate::search::evaluation::root_heuristics::opponent_tactics::opponent_knight_check_fork_penalty(
        post_after,
        side,
        to,
    )
}

pub fn queen_kingside_pressure_bonus(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    crate::search::evaluation::root_heuristics::queen_pressure::queen_kingside_pressure_bonus(
        base_board,
        side,
        from,
        to,
    )
}

pub fn threat_resolution_and_evacuation(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    gives_check: bool,
) -> i32 {
    crate::search::evaluation::root_heuristics::threat_resolution::threat_resolution_and_evacuation(
        base_board,
        post_after,
        side,
        from,
        to,
        gives_check,
    )
}

pub fn root_move_bonus(board: &Board, from: (usize, usize), to: (usize, usize), side: Color) -> i32 {
    crate::search::management::root_moves::root_move_bonus(board, from, to, side)
}

pub fn adjusted_root_eval_for_move(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    base_hmc: u32,
    score_raw: i32,
    is_capture: bool,
    moved_is_pawn: bool,
) -> i32 {
    crate::search::management::root_moves::adjusted_root_eval_for_move(
        base_board,
        side,
        from,
        to,
        promo,
        base_hmc,
        score_raw,
        is_capture,
        moved_is_pawn,
    )
}

pub fn build_pv_for_root(
    game_state: &GameState,
    from: (usize, usize),
    to: (usize, usize),
    root_promo: Option<char>,
    tt: &TranspositionTable,
    max_len: usize,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    crate::search::management::root_moves::build_pv_for_root(
        game_state,
        from,
        to,
        root_promo,
        tt,
        max_len,
    )
}

pub fn should_late_move_prune(
    depth: usize,
    move_index: i32,
    is_quiet: bool,
    gives_check: bool,
    in_check: bool,
    is_pv: bool,
) -> bool {
    crate::search::core::alphabeta::should_late_move_prune(
        depth,
        move_index,
        is_quiet,
        gives_check,
        in_check,
        is_pv,
    )
}

pub fn check_extension_budget() -> u8 {
    crate::search::core::alphabeta::CHECK_EXTENSION_BUDGET
}

pub fn should_check_extend(depth: usize, in_check: bool, gives_check: bool) -> bool {
    crate::search::core::alphabeta::should_check_extend(depth, in_check, gives_check)
}

#[allow(clippy::too_many_arguments)]
pub fn is_singular_extension(
    ctx: &SearchContext,
    game_state: &mut GameState,
    depth: usize,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    to_move: Color,
    best_move: ((usize, usize), (usize, usize)),
    best_score: i32,
    moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> bool {
    crate::search::core::alphabeta::is_singular_extension(
        ctx,
        game_state,
        depth,
        ply,
        tt,
        rep_stack,
        to_move,
        best_move,
        best_score,
        moves,
        prev_move,
    )
}

pub fn init_rayon_pool_if_needed() {
    crate::search::integration::threading::init_rayon_pool_if_needed();
}
