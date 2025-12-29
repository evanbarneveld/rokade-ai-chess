use rand::{rng, Rng};
use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::board::evaluator::evaluate_position;
use crate::board::san_move::convert_move_to_san;
use crate::history::history::History;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;

/// Find the best move for the given game state, the search_depth, and the playing_strength
///
pub (crate) fn find_move(game_state: GameState, search_depth: usize, playing_strength:usize, history: History) -> Option<((usize, usize), (usize, usize))> {

    // collect all legal moves for the side to move
    let board = game_state.board();
    let active_color = game_state.active_color();
    let valid_moves = find_all_valid_moves(board, active_color);

    if valid_moves.is_empty() {
        return None;
    }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if search_depth == 0 { 1 } else { search_depth };
    
    // create a vector with the moves and the scores
    let mut move_table: Vec<((usize, usize), (usize, usize), i32)> = Vec::new();

    // initialize root alpha/beta for potential root-level cutoffs
    let mut alpha = i32::MIN + 1;
    let mut beta = i32::MAX - 1;

    for (from, to) in valid_moves.into_iter() {
        // simulate the move once on a cloned board
        let simulation_board = move_piece_on_board(&board, from, to);

        // recurse: after making a move, it's the opponent's turn and depth decreases
        let mut score = if search_depth <= 1 {
            evaluate_position(&simulation_board)
        } else {
            // alpha-beta search from the opponent's perspective
            alphabeta(&simulation_board, opposite_color(active_color), search_depth - 1, alpha, beta)
        };

        // Small root-only tiebreaker: prefer captures slightly to reduce equal scores at low depth.
        if let Some(captured) = board.get(to.0, to.1) {
            use crate::piece::pieces::PieceType::*;
            let cap_val = match captured.get_type() {
                Pawn => 100,
                Knight => 320,
                Bishop => 330,
                Rook => 500,
                Queen => 900,
                King => 0,
            };
            score += cap_val / 10; // small bonus for capturing more valuable pieces
        }

        move_table.push((from, to, score));

        // update root alpha/beta based on side to move
        if active_color == Color::White {
            if score > alpha { alpha = score; }
        } else {
            if score < beta { beta = score; }
        }

        // optional root-level cutoff: if bounds cross, remaining moves unlikely to change decision
        if alpha >= beta { break; }
    }

    if move_table.is_empty() { return None; }


    let mut sorted_moves = sort_moves_on_score_asc(&mut move_table);

    // For White (maximizing side), higher scores are better. We sorted ascending,
    // so reverse to get best-first ordering. For Black (minimizing) keep ascending.
    if active_color == Color::White {
        sorted_moves.reverse();
    }

    select_move_based_using_strength(&sorted_moves, playing_strength)
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
fn alphabeta(board: &Board, to_move: Color, depth: usize, mut alpha: i32, mut beta: i32) -> i32 {

    // print board
    //println!("alpha-beta\n{}", board.get_board_display_string(None));

    if depth == 0 {
        let score = evaluate_position(board);
        //println!("score: {}", score);
        return score;
    }

    let moves = find_all_valid_moves(board, to_move);
    if moves.is_empty() {
        let score = evaluate_position(board);
        //println!("score: {}", score);
        return score;
    }

    if to_move == Color::White {
        let mut value = i32::MIN;
        for (from, to) in moves.into_iter() {
            let b = move_piece_on_board(board, from, to);
            let score = alphabeta(&b, Color::Black, depth - 1, alpha, beta);
            if score > value { value = score; }
            if value > alpha { alpha = value; }
            if alpha >= beta { break; }
        }
        value
    } else {
        let mut value = i32::MAX;
        for (from, to) in moves.into_iter() {
            let b = move_piece_on_board(board, from, to);
            let score = alphabeta(&b, Color::White, depth - 1, alpha, beta);
            if score < value { value = score; }
            if value < beta { beta = value; }
            if alpha >= beta { break; }
        }
        value
    }
}

fn move_piece_on_board(current: &Board, from: (usize, usize), to: (usize, usize)) -> Board {
    let mut clone = current.clone();
    // move the piece on the cloned board (no special moves handling here)
    // if it's a pawn, use move_pawn API; otherwise generic move_piece
    if let Some(p) = clone.get(from.0, from.1) {
        if p.get_type() == PieceType::Pawn {
            let _ = clone.move_pawn(from, to, None);
        } else {
            let _ = clone.move_piece(from, to);
        }
    }
    clone
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
