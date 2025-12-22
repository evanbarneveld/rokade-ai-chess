use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/*
 Given a incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_pawn_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &Board, active_color:Color) -> Option<(u8, u8)> {
    // Convert destination to indexes
    if to_col < 0 || to_row < 0 { return None; }
    let to_col_u = to_col as usize;
    let to_row_u = to_row as usize;

    let dir: i8 = match active_color {
        Color::White => 1,
        Color::Black => -1,
    };

    let start_row: i8 = match active_color {
        Color::White => 1, // rank 2 -> row index 1
        Color::Black => 6, // rank 7 -> row index 6
    };

    let mut candidates: Vec<(usize, usize)> = Vec::new(); // (row, col)

    // Helper to check if a board square has our move_validators
    let has_our_pawn = |r: i8, c: i8| -> bool {
        if r < 0 || c < 0 || r > 7 || c > 7 { return false; }
        if let Some(p) = board.get(r as usize, c as usize) {
            p.get_type() == PieceType::Pawn && p.get_color() == active_color
        } else {
            false
        }
    };

    // Respect disambiguation constraints, if provided (-1 means unknown)
    let col_matches = |c: i8| -> bool { from_col == -1 || from_col == c };
    let row_matches = |r: i8| -> bool { from_row == -1 || from_row == r };

    if is_capture {
        // Pawn capture must be diagonal one step
        let from_r = to_row - dir;
        for dc in [-1, 1] {
            let from_c = to_col + dc;
            if !col_matches(from_c) || !row_matches(from_r) { continue; }

            if has_our_pawn(from_r, from_c) {
                // target must have opponent piece (ignoring en passant for now)
                if let Some(target_piece) = board.get(to_row_u, to_col_u) {
                    if target_piece.get_color() != active_color {
                        candidates.push((from_r as usize, from_c as usize));
                    }
                }
            }
        }
    } else {
        // Non-capture: forward move by 1
        let from_r1 = to_row - dir;
        let from_c1 = to_col;
        let path_clear_1 = board.get(to_row_u, to_col_u).is_none();
        if path_clear_1 && col_matches(from_c1) && row_matches(from_r1) && has_our_pawn(from_r1, from_c1) {
            candidates.push((from_r1 as usize, from_c1 as usize));
        }

        // Non-capture: possible 2-step from starting rank
        let from_r2 = to_row - 2 * dir;
        let from_c2 = to_col;
        // must be from starting row and destination row exactly two ahead
        if from_r2 == start_row && col_matches(from_c2) && row_matches(from_r2) && has_our_pawn(from_r2, from_c2) {
            // squares must be empty: intermediate and target
            let mid_row = (to_row - dir) as usize;
            if board.get(to_row_u, to_col_u).is_none() && board.get(mid_row, to_col_u).is_none() {
                candidates.push((from_r2 as usize, from_c2 as usize));
            }
        }
    }

    // Ensure unique resolution
    if candidates.len() == 1 {
        let (r, c) = candidates[0];
        // move_to_string expects (file_index, rank_number[1..8])
        let file = c as u8;
        let rank_number = (r as u8) + 1; // convert 0-based row to human rank
        Some((file, rank_number))
    } else {
        None
    }
}

