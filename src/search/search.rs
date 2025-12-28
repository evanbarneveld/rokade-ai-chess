use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::board::evaluator::evaluate_position;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, PieceType};


pub (crate) fn find_best_move(board: &Board, active_color:Color) -> Option<((usize, usize), (usize, usize))> {

    // get all valid moves given the position on the board and the active_color
    let moves = find_valid_moves(board, active_color);

    // for each valid move, simulate the move on the board, evaluate_position and then restore the board to its original state
    // select the move with the best evaluation and return it, except if there is none
    if moves.is_empty() {
        return None;
    }

    let mut best_move: Option<((usize, usize), (usize, usize))> = None;
    let mut best_score: Option<i32> = None;

    for (from, to) in moves.into_iter() {
        // simulate the move
        let simulation_board = simulate_on_cloned_board(&board, from, to);
        let score = evaluate_position(&simulation_board);

        match best_score {
            None => {
                best_score = Some(score);
                best_move = Some((from, to));
            }
            Some(current) => {
                if score > current {
                    best_score = Some(score);
                    best_move = Some((from, to));
                }
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

