use crate::board::Board;
use crate::board::evaluation_helpers::{count_bishop_mobility, taper_general};
use crate::piece::pieces::{Color, PieceType};

const BISHOP_DEV_BONUS: i32 = 6;
const BISHOP_MOBILITY_BASELINE: i32 = 6;  // Average bishop has ~6 safe squares
const BISHOP_MOBILITY_MG: i32 = 5;        // Centipawns per safe square above baseline
const BISHOP_MOBILITY_EG: i32 = 4;        // Bishops remain important in endgame
const BISHOP_UNSAFE_PENALTY: i32 = 2;     // Penalty per square only reachable unsafely

pub fn evaluate_bishop(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;

    let home = match color {
        Color::White => row == 0 && (col == 2 || col == 5),
        Color::Black => row == 7 && (col == 2 || col == 5),
    };

    // Safe mobility evaluation (only for developed bishops)
    let (total, safe) = count_bishop_mobility(board, row, col, color);
    if !home {
        let mobility_delta = safe - BISHOP_MOBILITY_BASELINE;
        val += taper_general(mobility_delta * BISHOP_MOBILITY_MG, mobility_delta * BISHOP_MOBILITY_EG, phase);

        // Penalty for squares only reachable unsafely
        let unsafe_squares = total - safe;
        if unsafe_squares > 0 {
            val -= (unsafe_squares * BISHOP_UNSAFE_PENALTY * phase) / 24;
        }
    }

    if phase > 0 {
        if !home && total >= 2 {
            val += (BISHOP_DEV_BONUS * phase) / 24;
        }

        // Bad bishop penalty (many pawns on same color squares)
        let same_color_pawns = count_same_color_pawns(board, color, (row + col) % 2 == 0);
        if same_color_pawns >= 4 {
            let mut penalty = (same_color_pawns - 3) * 4;
            if safe <= 4 {
                penalty += 4;  // Extra penalty if mobility is also restricted
            }
            val -= (penalty * phase) / 24;
        }
    }
    val
}

fn count_same_color_pawns(board: &Board, color: Color, dark_square: bool) -> i32 {
    let mut count = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color
                && p.get_type() == PieceType::Pawn
                && ((r + c) % 2 == 0) == dark_square
            {
                count += 1;
            }
        }
    }
    count
}
