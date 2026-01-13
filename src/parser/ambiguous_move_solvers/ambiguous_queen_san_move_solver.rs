use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::piece::pieces::{Color, PieceType};

/*
Given an incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_queen_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &mut Board, active_color:Color) -> Option<(u8, u8)> {
    // Validate destination bounds
    if to_col < 0 || to_row < 0 { return None; }
    if to_col > 7 || to_row > 7 { return None; }

    let to_col_u = to_col as usize;
    let to_row_u = to_row as usize;

    // Disambiguation helpers (-1 means unknown)
    let col_matches = |c: i8| -> bool { from_col == -1 || from_col == c };
    let row_matches = |r: i8| -> bool { from_row == -1 || from_row == r };

    // Validate target occupancy depending on capture flag
    let target_ok = if is_capture {
        board.has_opposite_color_piece((to_row_u, to_col_u), active_color)
    } else {
        board.is_empty((to_row_u, to_col_u))
    };
    if !target_ok { return None; }

    let mut candidates: Vec<(usize, usize)> = Vec::new(); // (row, col)

    // Queen moves along 8 directions (rook + bishop)
    let directions: [(i8, i8); 8] = [
        // Rook-like
        (-1, 0), (1, 0), (0, -1), (0, 1),
        // Bishop-like
        (-1, -1), (-1, 1), (1, -1), (1, 1),
    ];

    for (dr, dc) in directions.iter() {
        let mut r = to_row + dr;
        let mut c = to_col + dc;
        while (0..=7).contains(&r) && (0..=7).contains(&c) {
            let ru = r as usize;
            let cu = c as usize;

            if let Some(p) = board.get(ru, cu) {
                // First piece on the ray determines if a candidate exists
                if p.get_type() == PieceType::Queen && p.get_color() == active_color
                    && col_matches(c) && row_matches(r)
                        && !is_king_in_check_after_move(board, (ru, cu), (to_row_u, to_col_u), None) {
                            candidates.push((ru, cu));
                        }
                break; // blocked beyond first piece
            }

            r += dr;
            c += dc;
        }
    }

    if candidates.len() == 1 {
        let (r, c) = candidates[0];
        let file = c as u8; // 0..7
        let rank_number = (r as u8) + 1; // 1..8
        Some((file, rank_number))
    } else {
        None
    }
}

