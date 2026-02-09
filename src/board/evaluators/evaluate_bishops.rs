use crate::board::Board;
use crate::board::evaluation_helpers::{count_bishop_mobility, is_piece, opponent, taper_general};
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

    // Bishop outpost bonus (smaller than knight's +22/+8)
    if is_bishop_outpost(board, row, col, color) {
        val += taper_general(14, 6, phase);
    }

    val
}

/// Check if a bishop is on an outpost square.
/// Similar to knight outposts but with slightly less value since bishops can retreat more easily.
/// Requirements:
/// - On ranks 4-6 (White) / 2-4 (Black)
/// - Protected by a friendly pawn
/// - Cannot be attacked by enemy pawns on adjacent files ahead
pub fn is_bishop_outpost(board: &Board, row: usize, col: usize, color: Color) -> bool {
    // Check rank range for outpost (ranks 4-6 for white = rows 3-5, ranks 5-3 for black = rows 4-2)
    let (min_r, max_r) = match color {
        Color::White => (3, 5), // ranks 4-6
        Color::Black => (2, 4), // ranks 3-5
    };
    if row < min_r || row > max_r {
        return false;
    }

    // Check if protected by a friendly pawn
    let behind_row = match color {
        Color::White => row.checked_sub(1),
        Color::Black => if row < 7 { Some(row + 1) } else { None },
    };

    let mut protected = false;
    if let Some(br) = behind_row {
        for dc in [-1i32, 1] {
            let nc = col as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, br, nc as usize, color, PieceType::Pawn) {
                protected = true;
                break;
            }
        }
    }

    if !protected {
        return false;
    }

    // Check that no enemy pawns can attack this square from adjacent files ahead
    let enemy = opponent(color);
    if col > 0 {
        match enemy {
            Color::White => {
                // Enemy white pawns advance upward (lower rows attack higher rows)
                for r in 0..row {
                    if is_piece(board, r, col - 1, enemy, PieceType::Pawn) {
                        return false;
                    }
                }
            }
            Color::Black => {
                // Enemy black pawns advance downward (higher rows attack lower rows)
                for r in (row + 1)..8 {
                    if is_piece(board, r, col - 1, enemy, PieceType::Pawn) {
                        return false;
                    }
                }
            }
        }
    }
    if col < 7 {
        match enemy {
            Color::White => {
                for r in 0..row {
                    if is_piece(board, r, col + 1, enemy, PieceType::Pawn) {
                        return false;
                    }
                }
            }
            Color::Black => {
                for r in (row + 1)..8 {
                    if is_piece(board, r, col + 1, enemy, PieceType::Pawn) {
                        return false;
                    }
                }
            }
        }
    }

    true
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
