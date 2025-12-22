use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/*
Given a incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_queen_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &Board, active_color:Color) -> Option<(u8, u8)> {
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
        board.board_square_has_piece_of_opposite_color((to_row_u, to_col_u), active_color)
    } else {
        board.board_square_is_empty((to_row_u, to_col_u))
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
        while r >= 0 && r <= 7 && c >= 0 && c <= 7 {
            let ru = r as usize;
            let cu = c as usize;

            if let Some(p) = board.get(ru, cu) {
                // First piece on the ray determines if a candidate exists
                if p.get_type() == PieceType::Queen && p.get_color() == active_color {
                    if col_matches(c) && row_matches(r) {
                        candidates.push((ru, cu));
                    }
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

