//! Detect opponent tactical opportunities after our moves.

use crate::board::Board;
use crate::piece::pieces::{opposite_color, Color, PieceType};

use super::utils::apply_for_side;

/// Penalty for allowing opponent knight fork with check
const KNIGHT_CHECK_PENALTY: i32 = 300;
/// Extra penalty when the check also forks a valuable piece
const KNIGHT_FORK_BONUS_PENALTY: i32 = 700;

/// Detect if opponent has knight fork opportunities with check after our move.
/// This catches tactical oversights where a capture or move allows opponent
/// a strong knight check that forks valuable pieces - specifically pieces
/// that just moved or are near the moved piece.
#[inline]
pub fn opponent_knight_check_fork_penalty(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
) -> i32 {
    let opp = opposite_color(side);

    // Find opponent knights
    let mut opp_knights = Vec::new();
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = post_after.get(r, c) {
                if p.get_color() == opp && p.get_type() == PieceType::Knight {
                    opp_knights.push((r, c));
                }
            }
        }
    }

    if opp_knights.is_empty() {
        return 0;
    }

    // Find our king position
    let our_king = find_king_square(post_after, side);
    if our_king.is_none() {
        return 0;
    }
    let (king_r, king_c) = our_king.unwrap();

    let mut total_penalty = 0;

    // For each opponent knight, check all possible knight moves
    for (nr, nc) in opp_knights {
        let knight_moves = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
        ];

        for (dr, dc) in knight_moves {
            let r_i32 = nr as i32 + dr;
            let c_i32 = nc as i32 + dc;

            if r_i32 < 0 || r_i32 >= 8 || c_i32 < 0 || c_i32 >= 8 {
                continue;
            }

            let dest_r = r_i32 as usize;
            let dest_c = c_i32 as usize;

            // Check if destination is empty or can be captured
            let dest_piece = post_after.get(dest_r, dest_c);
            if dest_piece.is_some() && dest_piece.unwrap().get_color() == opp {
                continue; // Can't move to square occupied by own piece
            }

            // Check if this move would give check
            let king_dist_r = (dest_r as i32 - king_r as i32).abs();
            let king_dist_c = (dest_c as i32 - king_c as i32).abs();
            let gives_check = (king_dist_r == 2 && king_dist_c == 1) ||
                              (king_dist_r == 1 && king_dist_c == 2);

            if !gives_check {
                continue;
            }

            // This knight move would give check - check if it also attacks the piece that just moved (fork)
            // This is the critical case: we moved a piece to 'to', and now opponent can fork it with check
            let forks_moved_piece = {
                // Check if knight at dest would attack the square 'to' where we just moved
                let check_dist_r = (dest_r as i32 - to.0 as i32).abs();
                let check_dist_c = (dest_c as i32 - to.1 as i32).abs();
                let attacks_dest = (check_dist_r == 2 && check_dist_c == 1) ||
                                  (check_dist_r == 1 && check_dist_c == 2);

                if attacks_dest {
                    // Check if there's a valuable piece on 'to'
                    if let Some(p) = post_after.get(to.0, to.1) {
                        p.get_color() == side && (p.get_type() == PieceType::Queen ||
                                                  p.get_type() == PieceType::Rook ||
                                                  p.get_type() == PieceType::Bishop ||
                                                  p.get_type() == PieceType::Knight)
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            // Apply penalty if opponent can fork our piece that just moved with check
            // This is a critical tactical oversight
            if gives_check && forks_moved_piece {
                total_penalty += KNIGHT_CHECK_PENALTY + KNIGHT_FORK_BONUS_PENALTY;
            }
        }
    }

    apply_for_side(-total_penalty, side)
}

/// Find the king square on a board for a given color.
#[inline]
fn find_king_square(board: &Board, color: Color) -> Option<(usize, usize)> {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                if p.get_color() == color && p.get_type() == PieceType::King {
                    return Some((r, c));
                }
            }
        }
    }
    None
}
