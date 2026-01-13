use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/*
 Given an incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_king_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &mut Board, active_color:Color) -> Option<(u8, u8)> {
    // Validate target bounds
    if to_col < 0 || to_row < 0 { return None; }
    if to_col > 7 || to_row > 7 { return None; }

    let to_col_u = to_col as usize;
    let to_row_u = to_row as usize;

    // Disambiguation helpers (-1 means unknown)
    let col_matches = |c: i8| -> bool { from_col == -1 || from_col == c };
    let row_matches = |r: i8| -> bool { from_row == -1 || from_row == r };

    // Target square must satisfy capture flag
    let target_ok = if is_capture {
        board.has_opposite_color_piece((to_row_u, to_col_u), active_color)
    } else {
        board.is_empty((to_row_u, to_col_u))
    };
    if !target_ok { return None; }

    // Check adjacent 8 squares for our king as possible source
    let mut candidates: Vec<(usize, usize)> = Vec::new(); // (row, col)

    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 { continue; }

            let from_r = to_row - dr; // invert delta to find source
            let from_c = to_col - dc;

            if from_r < 0 || from_r > 7 || from_c < 0 || from_c > 7 { continue; }
            if !col_matches(from_c) || !row_matches(from_r) { continue; }

            let from_r_u = from_r as usize;
            let from_c_u = from_c as usize;
            if let Some(p) = board.get(from_r_u, from_c_u) {
                if p.get_type() == PieceType::King && p.get_color() == active_color {
                    candidates.push((from_r_u, from_c_u));
                }
            }
        }
    }

    if candidates.len() == 1 {
        let (r, c) = candidates[0];
        let file = c as u8;
        let rank_number = (r as u8) + 1; // 0-based row to 1-based rank
        Some((file, rank_number))
    } else {
        None
    }
}

