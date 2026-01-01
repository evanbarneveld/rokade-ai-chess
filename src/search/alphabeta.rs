use std::sync::{Mutex, OnceLock};
use crate::board::Board;
use crate::board::evaluator::evaluate_position;
use crate::piece::pieces::{Color, PieceType};
use crate::search::heuristics::SearchHeuristics;
use crate::search::prune_null_moves::prune_null_moves;
use crate::search::qsearch::qsearch;
use crate::search::search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use crate::search::telemetry::bump_node;
use crate::search::time_control::time_is_up;
use crate::search::tt::{decode_move, encode_move, from_tt_score, to_tt_score, Bound, TranspositionTable, MATE_VALUE};
use crate::search::zobrist::compute_zobrist;

const HUNDRED_HALF_MOVES: u32 = 100;

// Alpha-beta pruning search. Returns evaluation in centipawns (positive is better for White).
pub fn alphabeta(
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
        return evaluate_position(&*board, to_move);
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
    if halfmove_clock >= HUNDRED_HALF_MOVES {
        return 0;
    }

    if depth == 0 {
        // At leaf: switch to quiescence to avoid horizon effects
        return qsearch(board, to_move, alpha, beta, halfmove_clock, rep_stack);
    }

    if let Some(value) = prune_null_moves(board, to_move, depth, beta, ply, tt, halfmove_clock, rep_stack) {
        return value;
    }

    // TT probe
    let key = key_here;
    if let Some(entry) = tt.probe(key) {
        if entry.depth as usize >= depth {
            let tt_score = from_tt_score(entry.score, ply);
            match entry.bound {
                Bound::Exact => {
                    // Only EXACT entries may short-circuit
                    return tt_score;
                }
                Bound::Lower => {
                    // Do not return early on LOWER; only tighten alpha
                    if tt_score > alpha {
                        alpha = tt_score;
                    }
                }
                Bound::Upper => {
                    // Do not return early on UPPER; only tighten beta
                    if tt_score < beta {
                        beta = tt_score;
                    }
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
            let mut key = board_ref.move_score_mvv_lva(from, to);
            let moved_is_pawn = board_ref
                .get(from.0, from.1)
                .map(|p| p.get_type() == PieceType::Pawn)
                .unwrap_or(false);
            let is_capture = board_ref.get(to.0, to.1).is_some();
            // DTZ-like ordering bump: near 50-move horizon, prioritize pawn moves and captures
            if hmc >= 80 {
                if moved_is_pawn || is_capture {
                    key += 100_000;
                }
            }
            // Killer move bonus (only for quiets)
            if !is_capture && !moved_is_pawn {
                let is_killer = HEUR.with(|h| {
                    let m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.is_killer(ply as usize, from, to)
                });
                if is_killer {
                    key += 200_000;
                }
                // History bonus
                let hist = HEUR.with(|h| {
                    let m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
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
        let in_check = board.is_side_in_check(to_move);
        return if in_check {
            // Losing side to move is checkmated. Use large negative for side to move.
            // Depth-based bonus (the sooner the mate, the larger the magnitude):
            // With our interface lacking ply, approximate using remaining depth.
            -MATE_VALUE + depth as i32
        } else {
            // stalemate: draw
            0
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
        let mut move_index: i32 = 0; // LMR: track move order
        // Precompute whether our queen is currently under attack in this node (before making a move)
        let mut queen_in_danger = false;
        {
            use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
            'findq: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() == to_move && p.get_type() == PieceType::Queen {
                            if is_square_attacked_by_opponent(board, (r, c), to_move) {
                                queen_in_danger = true;
                            }
                            break 'findq;
                        }
                    }
                }
            }
        }
        for (from, to) in moves.into_iter() {
            // Detect a moved piece before making the move
            let moved_piece = board.get(from.0, from.1);
            let target_piece = board.get(to.0, to.1);
            let u = board.make_move_simple(from, to);
            // Passed-pawn push extension (B5): If a pawn move results in a passed pawn
            // reaching the 6th/7th rank (relative to the side) in a near-endgame, extend by +1 ply.
            let mut child_depth = depth.saturating_sub(1);
            // Track halfmove clock: reset on pawn move or capture
            let mut child_hmc = halfmove_clock + 1;
            if let Some(p) = moved_piece {
                if p.get_type() == PieceType::Pawn {
                    child_hmc = 0;
                    let color = p.get_color();
                    let r = to.0;
                    let c = to.1;
                    if board.game_phase_light() <= 8 && board.is_passed_pawn_simple(r, c, color) {
                        let adv: i32 = match color {
                            Color::White => r as i32,
                            Color::Black => (7 - r) as i32,
                        };
                        if adv >= 5 {
                            // 6th or 7th rank
                            child_depth = child_depth.saturating_add(1);
                        }
                    }
                }
            }
            if target_piece.is_some() {
                child_hmc = 0;
            }

            // History/Killer move ordering boost for quiet moves (after initial TT/MVV ordering)
            let is_capture = target_piece.is_some();
            let is_pawn_move = moved_piece
                .map(|p| p.get_type() == PieceType::Pawn)
                .unwrap_or(false);
            let quiet = !is_capture && !is_pawn_move;

            // Principal Variation Search (PVS) + Late Move Reductions (LMR)
            let mut score;
            if is_first_move {
                // Full window on the first move
                score = alphabeta(
                    board,
                    Color::Black,
                    child_depth,
                    alpha,
                    beta,
                    ply + 1,
                    tt,
                    child_hmc,
                    rep_stack,
                );
                is_first_move = false;
            } else {
                // Late Move Reduction (safer): start later and cap reductions
                let mut reduced_depth = child_depth;
                // Do not reduce if the move gives check or if our queen is currently in danger (urgent moves)
                // Also, do not reduce if this move is a queen move landing on an attacked square — needs full depth.
                let gives_check = board.is_side_in_check(Color::Black);
                // Strictly avoid reducing checking moves and immediate recaptures at shallow depth
                let mut queen_into_danger = false;
                if let Some(p) = moved_piece {
                    if p.get_type() == PieceType::Queen {
                        use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                        let attacked = is_square_attacked_by_opponent(board, to, Color::White);
                        queen_into_danger = attacked;
                    }
                }
                // Never reduce checking moves at shallow depths
                let allow_reduce = !(gives_check && child_depth <= 5);
                // If any queen is attacked at this node and depth is shallow, avoid LMR for quiets
                let mut any_queen_attacked_here = false;
                {
                    use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                    // White queen
                    'qw: for r in 0..8 {
                        for c in 0..8 {
                            if let Some(p) = board.get(r, c) {
                                if p.get_type() == PieceType::Queen && p.get_color() == Color::White
                                {
                                    if is_square_attacked_by_opponent(board, (r, c), Color::White) {
                                        any_queen_attacked_here = true;
                                    }
                                    break 'qw;
                                }
                            }
                        }
                    }
                    // Black queen
                    if !any_queen_attacked_here {
                        'qb: for r in 0..8 {
                            for c in 0..8 {
                                if let Some(p) = board.get(r, c) {
                                    if p.get_type() == PieceType::Queen
                                        && p.get_color() == Color::Black
                                    {
                                        if is_square_attacked_by_opponent(
                                            board,
                                            (r, c),
                                            Color::Black,
                                        ) {
                                            any_queen_attacked_here = true;
                                        }
                                        break 'qb;
                                    }
                                }
                            }
                        }
                    }
                }
                let avoid_lmr_for_queen_threat = any_queen_attacked_here && child_depth <= 5;
                if quiet
                    && child_depth >= 3
                    && move_index >= 4
                    && allow_reduce
                    && !queen_in_danger
                    && !queen_into_danger
                    && !avoid_lmr_for_queen_threat
                {
                    // Basic reduction formula: grows with move index and depth
                    // Use history to avoid over-reducing historically good moves
                    let hist = HEUR.with(|h| {
                        let m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.history_score(to_move, from, to)
                    });
                    let hist_good = hist > 10_000; // tuned threshold
                    // Slightly more aggressive with stronger eval; allow up to 3 plies at depth >= 8
                    let mut r = 1 + ((move_index as usize) / 6).min(1);
                    if child_depth >= 8 {
                        r += 1;
                    }
                    if hist_good {
                        r = r.saturating_sub(1);
                    }
                    // Final cap to 3 plies
                    r = r.min(3);
                    reduced_depth = reduced_depth.saturating_sub(r);
                }
                // Null-window search for subsequent moves (PVS window)
                score = alphabeta(
                    board,
                    Color::Black,
                    reduced_depth,
                    alpha,
                    alpha + 1,
                    ply + 1,
                    tt,
                    child_hmc,
                    rep_stack,
                );
                if score > alpha && score < beta {
                    // Re-search with full window on fail-high/improvement inside window
                    score = alphabeta(
                        board,
                        Color::Black,
                        child_depth,
                        alpha,
                        beta,
                        ply + 1,
                        tt,
                        child_hmc,
                        rep_stack,
                    );
                }
            }
            board.unmake_move_simple(u);
            if score > value {
                value = score;
            }
            if value > alpha {
                alpha = value;
                best_from_to = Some((from, to));
                // On alpha improvement, update history for quiets
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score >= beta {
                // Beta cutoffs are handled by loop break, but record killer before break
                HEUR.with(|h| {
                    let mut m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
            if alpha >= beta {
                break;
            }
            move_index += 1;
        }
        value_holder = value;
        value_holder
    } else {
        let mut value = MAX_EVAL_VALUE;
        let mut is_first_move = true; // PVS for minimizing side too
        let mut move_index: i32 = 0; // LMR index
        // Precompute whether our queen is currently under attack in this node
        let mut queen_in_danger = false;
        {
            use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
            'findq: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() == to_move && p.get_type() == PieceType::Queen {
                            if is_square_attacked_by_opponent(board, (r, c), to_move) {
                                queen_in_danger = true;
                            }
                            break 'findq;
                        }
                    }
                }
            }
        }
        for (from, to) in moves.into_iter() {
            // Detect moved piece before making the move
            let moved_piece = board.get(from.0, from.1);
            let target_piece = board.get(to.0, to.1);
            let u = board.make_move_simple(from, to);
            // Passed-pawn push extension (B5)
            let mut child_depth = depth.saturating_sub(1);
            // Track halfmove clock for child
            let mut child_hmc = halfmove_clock + 1;
            if let Some(p) = moved_piece {
                if p.get_type() == PieceType::Pawn {
                    child_hmc = 0;
                    let color = p.get_color();
                    let r = to.0;
                    let c = to.1;
                    if board.game_phase_light() <= 8 && board.is_passed_pawn_simple(r, c, color) {
                        let adv: i32 = match color {
                            Color::White => r as i32,
                            Color::Black => (7 - r) as i32,
                        };
                        if adv >= 5 {
                            child_depth = child_depth.saturating_add(1);
                        }
                    }
                }
            }
            if target_piece.is_some() {
                child_hmc = 0;
            }
            // PVS + LMR for minimizing side
            let is_capture = target_piece.is_some();
            let is_pawn_move = moved_piece
                .map(|p| p.get_type() == PieceType::Pawn)
                .unwrap_or(false);
            let quiet = !is_capture && !is_pawn_move;
            let mut score;
            if is_first_move {
                score = alphabeta(
                    board,
                    Color::White,
                    child_depth,
                    alpha,
                    beta,
                    ply + 1,
                    tt,
                    child_hmc,
                    rep_stack,
                );
                is_first_move = false;
            } else {
                // Late Move Reduction (safer): start later and cap reductions
                let mut reduced_depth = child_depth;
                let gives_check = board.is_side_in_check(Color::White);
                // Strictly avoid reducing checking moves and immediate recaptures at shallow depth
                let mut queen_into_danger = false;
                if let Some(p) = moved_piece {
                    if p.get_type() == PieceType::Queen {
                        use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                        let attacked = is_square_attacked_by_opponent(board, to, Color::Black);
                        queen_into_danger = attacked;
                    }
                }
                // Never reduce checking moves at shallow depths
                let allow_reduce = !(gives_check && child_depth <= 5);
                // If any queen is attacked at this node and depth is shallow, avoid LMR for quiets
                let mut any_queen_attacked_here = false;
                {
                    use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                    'qw: for r in 0..8 {
                        for c in 0..8 {
                            if let Some(p) = board.get(r, c) {
                                if p.get_type() == PieceType::Queen && p.get_color() == Color::White
                                {
                                    if is_square_attacked_by_opponent(board, (r, c), Color::White) {
                                        any_queen_attacked_here = true;
                                    }
                                    break 'qw;
                                }
                            }
                        }
                    }
                    if !any_queen_attacked_here {
                        'qb: for r in 0..8 {
                            for c in 0..8 {
                                if let Some(p) = board.get(r, c) {
                                    if p.get_type() == PieceType::Queen
                                        && p.get_color() == Color::Black
                                    {
                                        if is_square_attacked_by_opponent(
                                            board,
                                            (r, c),
                                            Color::Black,
                                        ) {
                                            any_queen_attacked_here = true;
                                        }
                                        break 'qb;
                                    }
                                }
                            }
                        }
                    }
                }
                let avoid_lmr_for_queen_threat = any_queen_attacked_here && child_depth <= 5;
                if quiet
                    && child_depth >= 3
                    && move_index >= 4
                    && allow_reduce
                    && !queen_in_danger
                    && !queen_into_danger
                    && !avoid_lmr_for_queen_threat
                {
                    let hist = HEUR.with(|h| {
                        let m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.history_score(to_move, from, to)
                    });
                    let hist_good = hist < -10_000; // minimizing side: keep symmetric magnitude
                    let mut r = 1 + ((move_index as usize) / 6).min(1);
                    if (child_depth as usize) >= 8 {
                        r += 1;
                    }
                    if hist_good {
                        r = r.saturating_sub(1);
                    }
                    r = r.min(3);
                    reduced_depth = reduced_depth.saturating_sub(r);
                }
                // For the minimizing side, a null-window around beta-1 .. beta works equivalently
                score = alphabeta(
                    board,
                    Color::White,
                    reduced_depth,
                    beta - 1,
                    beta,
                    ply + 1,
                    tt,
                    child_hmc,
                    rep_stack,
                );
                if score < beta && score > alpha {
                    score = alphabeta(
                        board,
                        Color::White,
                        child_depth,
                        alpha,
                        beta,
                        ply + 1,
                        tt,
                        child_hmc,
                        rep_stack,
                    );
                }
            }
            board.unmake_move_simple(u);
            if score < value {
                value = score;
            }
            if value < beta {
                beta = value;
                best_from_to = Some((from, to));
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score <= alpha {
                HEUR.with(|h| {
                    let mut m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
            if alpha >= beta {
                break;
            }
            move_index += 1;
        }
        value_holder = value;
        value_holder
    };

    // Pop this node key
    let _ = rep_stack.pop();

    // Store to TT
    let bound = if value <= original_alpha {
        Bound::Upper
    } else if value >= original_beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    let (bf, bt) = if let Some((f, t)) = best_from_to {
        let (ff, tt2) = encode_move(f, t);
        (Some(ff), Some(tt2))
    } else {
        (None, None)
    };
    let tt_score = to_tt_score(value, ply);
    tt.store(key, depth as i16, bound, tt_score, bf, bt);
    value
}
