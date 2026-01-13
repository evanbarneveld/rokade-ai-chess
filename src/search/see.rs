use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, piece_value_cp, Color, Piece, PieceType};

// Root SEE-based gating and penalties
pub const SEE_PENALTY_MIN_CP: i32 = 80; // min penalty for negative SEE (non-queen)
pub const SEE_PENALTY_MAX_CP: i32 = 300; // max penalty for negative SEE (non-queen)

// SEE penalty scaling by piece type
const MINOR_PAWN_ATTACK_PENALTY: i32 = 1200;

// Conservative SEE estimate on destination square.
// Returns net material from the perspective of the side that just moved.
// Positive => safe/profitable, Negative => likely losing material on dest.
#[inline]
pub fn see_dest_estimate(
    board_after: &Board,
    side_just_moved: Color,
    dest: (usize, usize),
    captured_val: i32,
) -> i32 {
    use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
    let moved_piece = match board_after.get(dest.0, dest.1) {
        Some(p) => p,
        None => return captured_val,
    };
    let moved_val = piece_value_cp(moved_piece.get_type());
    // We need a mutable board for the attack probe helper; clone cheaply
    let mut tmp1 = *board_after;
    let attacked_by_opp = is_square_attacked_by_opponent(&mut tmp1, dest, side_just_moved);
    // defend check: reuse helper by swapping perspective
    let mut tmp2 = *board_after;
    let defended_by_us =
        is_square_attacked_by_opponent(&mut tmp2, dest, opposite_color(side_just_moved));

    if !attacked_by_opp {
        // No immediate opponent attack: return captured gain (if any)
        return captured_val;
    }
    if attacked_by_opp && !defended_by_us {
        // Likely losing the moved piece outright
        return captured_val - moved_val;
    }
    // Both attacked and defended: assume partial liability
    captured_val - (moved_val / 2)
}

// ============================================================
// SEE HELPER FUNCTIONS
// ============================================================

/// Apply sign based on side (positive for White, negative for Black).
#[inline]
fn apply_for_side(v: i32, side: Color) -> i32 {
    if side == Color::White { v } else { -v }
}

/// Compute SEE estimate after simulating a move.
#[inline]
pub fn see_after(board: &Board, side: Color, to: (usize, usize), captured: Option<Piece>) -> i32 {
    let cap = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
    see_dest_estimate(board, side, to, cap)
}

/// Check if a square is attacked by an opponent pawn.
#[inline]
pub fn attacked_by_pawn(board: &Board, sq: (usize, usize), attacker: Color) -> bool {
    let (r, c) = sq;
    match attacker {
        Color::White => {
            (r > 0 && c > 0 && matches!(board.get(r-1, c-1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
            || (r > 0 && c + 1 < 8 && matches!(board.get(r-1, c+1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
        }
        Color::Black => {
            (r + 1 < 8 && c > 0 && matches!(board.get(r+1, c-1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
            || (r + 1 < 8 && c + 1 < 8 && matches!(board.get(r+1, c+1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
        }
    }
}

/// Find the king square on a board for a given color.
#[inline]
fn find_king_square(board: &Board, color: Color) -> Option<(usize, usize)> {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color && p.get_type() == PieceType::King {
                    return Some((r, c));
                }
        }
    }
    None
}

/// Check if two squares are adjacent (within 1 step in any direction).
#[inline]
fn squares_adjacent(a: (usize, usize), b: (usize, usize)) -> bool {
    let dr = a.0.abs_diff(b.0);
    let dc = a.1.abs_diff(b.1);
    dr <= 1 && dc <= 1
}

// ============================================================
// SEE PENALTY HEURISTICS
// ============================================================

/// Check if opponent king can safely capture a piece on the destination square.
#[inline]
pub fn king_can_safely_capture(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
    moved_pt: PieceType,
) -> Option<i32> {
    let opp = opposite_color(side);

    // Find opponent king
    let king_sq = find_king_square(post_after, opp)?;

    // King can only capture if adjacent
    if !squares_adjacent(king_sq, to) {
        return None;
    }

    // Ensure destination holds our piece
    let on_to = post_after.get(to.0, to.1)?;
    if on_to.get_color() != side {
        return None;
    }

    // Simulate king capture
    let mut after_kx = *post_after;
    after_kx.set(king_sq.0, king_sq.1, None);
    after_kx.set(to.0, to.1, Some(Piece::new(PieceType::King, opp)));

    // Check if king is safe after capture
    let mut tmp_chk = after_kx;
    let unsafe_for_king = is_square_attacked_by_opponent(&mut tmp_chk, to, opp);

    if unsafe_for_king {
        return None;
    }

    // Apply penalty scaled by piece importance
    let base_pen = match moved_pt {
        PieceType::Queen => 900,
        PieceType::Rook => 500,
        PieceType::Bishop | PieceType::Knight => 300,
        _ => 200,
    };
    let scale = match moved_pt {
        PieceType::Queen | PieceType::Rook => 8,
        PieceType::Bishop | PieceType::Knight => 10,
        _ => 8,
    };
    Some((base_pen * scale).clamp(2400, 8000))
}

/// Calculate SEE-based penalty for checking piece attacked by pawn.
#[inline]
pub fn pawn_attacked_minor_penalty(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
    moved_pt: PieceType,
) -> i32 {
    if moved_pt != PieceType::Knight && moved_pt != PieceType::Bishop {
        return 0;
    }
    let opp = opposite_color(side);
    if attacked_by_pawn(post_after, to, opp) {
        MINOR_PAWN_ATTACK_PENALTY
    } else {
        0
    }
}

/// Apply SEE-based penalties for destination square vulnerabilities.
#[inline]
pub fn apply_destination_see_penalties(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
    moved_is_pawn: bool,
    gives_check: bool,
    _moved_is_queen: bool,
) -> i32 {
    let mut delta = 0;
    let captured = base_board.get(to.0, to.1);

    if !gives_check {
        // Non-checking moves: simple SEE penalty
        let see = see_after(post_after, side, to, captured);
        if see < 0 {
            let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP);
            delta += apply_for_side(-pen, side);
            if !is_capture && moved_is_pawn {
                delta += apply_for_side(-SEE_PENALTY_MIN_CP, side);
            }
        }
        return delta;
    }

    // Checking moves: guard against suicidal checks
    let see = see_after(post_after, side, to, captured);
    let moved_pt = base_board.get(from.0, from.1).map(|p| p.get_type());

    if see < 0 {
        // Scale penalty by piece importance
        let pen = match moved_pt {
            Some(PieceType::Queen) => ((-see) * 6).clamp(600, 6000),
            Some(PieceType::Rook) => ((-see) * 4).clamp(3600, 6000),
            Some(PieceType::Bishop) | Some(PieceType::Knight) => ((-see) * 4).clamp(600, 3600),
            Some(PieceType::Pawn) => ((-see) * 2).clamp(200, 2000),
            _ => ((-see) * 3).clamp(300, 3000),
        };
        delta += apply_for_side(-pen, side);
    }

    // Additional penalty for minors attacked by pawn after check
    if let Some(pt) = moved_pt {
        delta += apply_for_side(-pawn_attacked_minor_penalty(post_after, side, to, pt), side);
    }

    // Check if opponent king can safely capture the checking piece
    if let Some(pt) = moved_pt
        && matches!(pt, PieceType::Rook | PieceType::Queen | PieceType::Bishop | PieceType::Knight)
            && let Some(pen) = king_can_safely_capture(post_after, side, to, pt) {
                delta += apply_for_side(-pen, side);
            }

    delta
}
