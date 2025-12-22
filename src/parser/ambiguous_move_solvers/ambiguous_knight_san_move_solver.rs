use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/*
Given a incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_knight_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &Board, active_color:Color) -> Option<(u8, u8)> {
    // Convert destination to indexes and validate bounds
    if to_col < 0 || to_row < 0 { return None; }
    if to_col > 7 || to_row > 7 { return None; }

    let to_col_u = to_col as usize;
    let to_row_u = to_row as usize;

    // Disambiguation helpers (-1 means unknown)
    let col_matches = |c: i8| -> bool { from_col == -1 || from_col == c };
    let row_matches = |r: i8| -> bool { from_row == -1 || from_row == r };

    // Helper to check if a board square has our knight
    let has_our_knight = |r: i8, c: i8| -> bool {
        if r < 0 || c < 0 || r > 7 || c > 7 { return false; }
        if let Some(p) = board.get(r as usize, c as usize) {
            p.get_type() == PieceType::Knight && p.get_color() == active_color
        } else {
            false
        }
    };

    // Validate target occupancy depending on capture flag
    let target_ok = if is_capture {
        board.board_square_has_piece_of_opposite_color((to_row_u, to_col_u), active_color)
    } else {
        board.board_square_is_empty((to_row_u, to_col_u))
    };
    if !target_ok { return None; }

    let mut candidates: Vec<(usize, usize)> = Vec::new(); // (row, col)

    // All knight move deltas (from -> to). To get from-source, subtract these from target
    let deltas: [(i8, i8); 8] = [
        (-2, -1), (-2, 1),
        (-1, -2), (-1, 2),
        (1, -2),  (1, 2),
        (2, -1),  (2, 1),
    ];

    for (dr, dc) in deltas.iter() {
        let from_r = to_row - dr; // invert delta to find source
        let from_c = to_col - dc;

        if !col_matches(from_c) || !row_matches(from_r) { continue; }

        if has_our_knight(from_r, from_c) {
            candidates.push((from_r as usize, from_c as usize));
        }
    }

    // Ensure unique resolution
    if candidates.len() == 1 {
        let (r, c) = candidates[0];
        let file = c as u8; // file index 0..7
        let rank_number = (r as u8) + 1; // convert 0-based row to rank 1..8
        Some((file, rank_number))
    } else {
        None
    }
}

