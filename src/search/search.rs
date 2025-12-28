use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::board::evaluator::evaluate_position;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, PieceType};


pub (crate) fn find_best_move(board: &Board, active_color: Color, depth: usize) -> Option<((usize, usize), (usize, usize))> {
    // collect all legal moves for the side to move
    let moves = find_valid_moves(board, active_color);

    if moves.is_empty() {
        return None;
    }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if depth == 0 { 1 } else { depth };

    let mut best_move: Option<((usize, usize), (usize, usize))> = None;
    // If it's White to move we maximize, if Black we minimize (evaluation is from White's perspective)
    let mut best_score: i32 = if active_color == Color::White { i32::MIN } else { i32::MAX };

    for (from, to) in moves.into_iter() {
        // simulate the move
        let simulation_board = simulate_on_cloned_board(&board, from, to);

        // recurse: after making a move, it's the opponent's turn and depth decreases
        let score = if search_depth <= 1 {
            evaluate_position(&simulation_board)
        } else {
            minimax(&simulation_board, opposite_color(active_color), search_depth - 1)
        };

        if active_color == Color::White {
            if score > best_score {
                best_score = score;
                best_move = Some((from, to));
            }
        } else {
            if score < best_score {
                best_score = score;
                best_move = Some((from, to));
            }
        }
    }

    best_move
}

pub(crate) fn find_valid_moves(board: &Board, active_color:Color) -> Vec<((usize, usize), (usize, usize))> {
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

                    if ok {
                        result.push((from, to));
                    }
                }
            }
        }
    }
    result
}

#[inline]
fn opposite_color(c: Color) -> Color {
    match c { Color::White => Color::Black, Color::Black => Color::White }
}

// Basic depth-limited minimax without alpha-beta pruning.
// Returns an evaluation in centipawns (positive better for White).
fn minimax(board: &Board, to_move: Color, depth: usize) -> i32 {
    if depth == 0 {
        return evaluate_position(board);
    }

    let moves = find_valid_moves(board, to_move);
    if moves.is_empty() {
        // no legal moves: fall back to static eval (no checkmate detection here)
        return evaluate_position(board);
    }

    if to_move == Color::White {
        let mut best = i32::MIN;
        for (from, to) in moves.into_iter() {
            let b = simulate_on_cloned_board(board, from, to);
            let val = minimax(&b, Color::Black, depth - 1);
            if val > best { best = val; }
        }
        best
    } else {
        let mut best = i32::MAX;
        for (from, to) in moves.into_iter() {
            let b = simulate_on_cloned_board(board, from, to);
            let val = minimax(&b, Color::White, depth - 1);
            if val < best { best = val; }
        }
        best
    }
}

fn simulate_on_cloned_board(current: &Board, from: (usize, usize), to: (usize, usize)) -> Board {
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

