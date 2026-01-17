use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{is_piece, taper_general};

pub fn evaluate_bishop(board: &Board, row: usize, col: usize, color: Color, phase: i32, eg: i32) -> i32 {
    let mut val = 0;

    // Development bonus
    if phase > 0 {
        let home = match color {
            Color::White => row == 0 && (col == 2 || col == 5),
            Color::Black => row == 7 && (col == 2 || col == 5),
        };
        if !home { val += (8 * phase) / 24; }
    }

    // Fianchetto bonus - bishops on developed fianchetto squares
    let is_fianchetto = match color {
        Color::White => row == 1 && (col == 1 || col == 6),
        Color::Black => row == 6 && (col == 1 || col == 6),
    };
    if is_fianchetto {
        // Check if the pawn is still there (proper fianchetto structure)
        let pawn_row = match color { Color::White => 2, Color::Black => 5 };
        let pawn_col = if col == 1 { 1 } else { 6 };
        if is_piece(board, pawn_row, pawn_col, color, PieceType::Pawn) {
            val += taper_general(phase, 18, 6);
        }
    }

    // Long diagonal bonus (a1-h8 or h1-a8 diagonals)
    let on_long_diagonal = (row == col) || (row + col == 7);
    if on_long_diagonal && !is_near_edge(row, col) {
        val += (6 * eg) / 24;
    }

    // Bad bishop penalty - blocked by own pawns
    val -= bad_bishop_penalty(board, row, col, color, phase);

    val
}

/// Count how many squares a bishop can reach
#[allow(dead_code)]
pub fn bishop_mobility(board: &Board, row: usize, col: usize, color: Color) -> usize {
    let mut count = 0;
    for (dr, dc) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut r = row as i32 + dr;
        let mut c = col as i32 + dc;
        while (0..8).contains(&r) && (0..8).contains(&c) {
            let ru = r as usize;
            let cu = c as usize;
            if let Some(p) = board.get(ru, cu) {
                if p.get_color() != color {
                    count += 1; // Can capture
                }
                break; // Blocked
            }
            count += 1;
            r += dr;
            c += dc;
        }
    }
    count
}

fn is_near_edge(row: usize, col: usize) -> bool {
    row <= 1 || row >= 6 || col <= 1 || col >= 6
}

/// Detect bad bishop: bishop blocked by own pawns on same color squares
fn bad_bishop_penalty(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let bishop_color = (row + col) % 2; // 0 = dark squares, 1 = light squares

    let mut own_pawns_on_bishop_color = 0;
    let mut central_blocked_pawns = 0;

    for r in 0..8 {
        for c in 0..8 {
            if (r + c) % 2 == bishop_color {
                if is_piece(board, r, c, color, PieceType::Pawn) {
                    own_pawns_on_bishop_color += 1;

                    // Extra penalty for central pawns that are blocked
                    if c == 3 || c == 4 {
                        let ahead = match color {
                            Color::White => if r < 7 { r + 1 } else { continue },
                            Color::Black => if r > 0 { r - 1 } else { continue },
                        };
                        if board.get(ahead, c).is_some() {
                            central_blocked_pawns += 1;
                        }
                    }
                }
            }
        }
    }

    // Penalty scales with number of own pawns on bishop's color
    let mut penalty = 0;
    if own_pawns_on_bishop_color >= 5 {
        penalty += taper_general(phase, 20, 12);
    } else if own_pawns_on_bishop_color >= 4 {
        penalty += taper_general(phase, 12, 6);
    }

    // Additional penalty for blocked central pawns
    penalty += central_blocked_pawns * taper_general(phase, 8, 4);

    penalty
}

/// Bishop pair bonus - should be called once per side in the main evaluator
pub fn bishop_pair_bonus(board: &Board, color: Color, phase: i32, eg: i32) -> i32 {
    let mut light_bishop = false;
    let mut dark_bishop = false;

    for r in 0..8 {
        for c in 0..8 {
            if is_piece(board, r, c, color, PieceType::Bishop) {
                if (r + c) % 2 == 0 {
                    dark_bishop = true;
                } else {
                    light_bishop = true;
                }
            }
        }
    }

    if light_bishop && dark_bishop {
        // Bishop pair is strong in all phases, but especially endgame
        (30 * phase + 50 * eg) / 24
    } else {
        0
    }
}
