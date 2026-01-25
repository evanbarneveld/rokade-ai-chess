use crate::board::Board;
use crate::board::evaluation_helpers::{chebyshev_dist, count_queen_mobility, is_piece, taper_general, PawnFileCounts};
use crate::piece::pieces::{Color, PieceType};

const QUEEN_CENTER_BONUS: i32 = 4;
const QUEEN_CENTER_MAX_DIST: i32 = 4;

// Queen mobility constants (lower weights - queen mobility is less critical)
const QUEEN_MOBILITY_BASELINE: i32 = 14;  // Queens have high mobility
const QUEEN_MOBILITY_MG: i32 = 1;         // Small bonus per safe square
const QUEEN_MOBILITY_EG: i32 = 2;         // Slightly more in endgame
const QUEEN_UNSAFE_PENALTY: i32 = 1;      // Penalty for unsafe squares

pub fn evaluate_queen(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;

    // Safe mobility evaluation
    let (total, safe) = count_queen_mobility(board, row, col, color);
    let mobility_delta = safe - QUEEN_MOBILITY_BASELINE;
    val += taper_general(mobility_delta * QUEEN_MOBILITY_MG, mobility_delta * QUEEN_MOBILITY_EG, phase);

    // Penalty for squares only reachable unsafely
    let unsafe_squares = total - safe;
    if unsafe_squares > 0 {
        val -= (unsafe_squares * QUEEN_UNSAFE_PENALTY * phase) / 24;
    }

    if phase > 0 {
        if col == 0 || col == 7 {
            let deep = match color { Color::White => row >= 3, Color::Black => row <= 4 };
            if deep { val -= (12 * phase) / 24; }
        }
        let (back_r, k1c, k2c) = match color { Color::White => (0, 1, 6), Color::Black => (7, 1, 6) };
        let both_knights_back = is_piece(board, back_r, k1c, color, PieceType::Knight)
            && is_piece(board, back_r, k2c, color, PieceType::Knight);
        if both_knights_back {
            let shallow = match color { Color::White => row <= 2, Color::Black => row >= 5 };
            if shallow { val -= (14 * phase) / 24; }
        }
    }
    let eg = 24 - phase;
    if eg > 0 {
        let mut best = 99;
        for (cr, cc) in [(3, 3), (3, 4), (4, 3), (4, 4)] {
            let dist = chebyshev_dist((row as i32, col as i32), (cr, cc));
            if dist < best {
                best = dist;
            }
        }
        let center_bonus = (QUEEN_CENTER_MAX_DIST - best).max(0) * QUEEN_CENTER_BONUS;
        val += (center_bonus * eg) / 24;
    }
    val
}

pub fn queen_on_semi_open_file_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    let mut score = 0;
    for c in 0..8 {
        let mut has_queen = false;
        for r in 0..8 { if is_piece(board, r, c, color, PieceType::Queen) { has_queen = true; break; } }
        if has_queen {
            let friendly = match color { Color::White => counts.white[c], Color::Black => counts.black[c] };
            if friendly == 0 { score += 6; }
        }
    }
    score
}

pub fn early_queen_penalty(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    let mut pen = 0;
    let mut queen_pos = None;
    for r in 0..8 {
        for c in 0..8 {
            if is_piece(board, r, c, color, PieceType::Queen) { queen_pos = Some((r, c)); break; }
        }
    }
    if let Some((r, c)) = queen_pos {
        let (home_row, start_col) = match color { Color::White => (0, 3), Color::Black => (7, 3) };
        if r != home_row || c != start_col {
            let mut minor_pieces_at_home = 0;
            let minors = match color { Color::White => [(0, 1), (0, 2), (0, 5), (0, 6)], Color::Black => [(7, 1), (7, 2), (7, 5), (7, 6)] };
            for &(mr, mc) in &minors {
                if let Some(p) = board.get(mr, mc)
                    && p.get_color() == color
                    && (p.get_type() == PieceType::Knight || p.get_type() == PieceType::Bishop)
                {
                    minor_pieces_at_home += 1;
                }
            }
            if minor_pieces_at_home >= 2 {
                pen += 15 * minor_pieces_at_home;
                let (_own_pawns, opp_pawns) = match color {
                    Color::White => (&counts.white, &counts.black),
                    Color::Black => (&counts.black, &counts.white),
                };
                let mut advanced = false;
                match color {
                    Color::White => if r >= 2 { advanced = true; },
                    Color::Black => if r <= 5 { advanced = true; },
                }
                if advanced {
                    let mut exposed = false;
                    for dc in -1..=1 {
                        let nc = c as i32 + dc;
                        if (0..=7).contains(&nc) {
                            let nc = nc as usize;
                            if opp_pawns[nc] > 0 { exposed = true; break; }
                        }
                    }
                    if exposed { pen += 20; }
                }
            }
        }
    }
    pen
}
