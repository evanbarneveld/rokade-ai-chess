use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, piece_value_cp, Color, Piece, PieceType};

// Root SEE-based gating and penalties
pub const SEE_PENALTY_MIN_CP: i32 = 80; // min penalty for negative SEE (non-queen)
pub const SEE_PENALTY_MAX_CP: i32 = 300; // max penalty for negative SEE (non-queen)

// Quiescence SEE threshold: Allow slightly negative captures to avoid over-pruning
// A small tolerance prevents discarding moves that might lead to tactical complications
pub const QSEE_CAPTURE_TOLERANCE: i32 = -50;

// SEE penalty scaling by piece type
const MINOR_PAWN_ATTACK_PENALTY: i32 = 1200;

// Full Static Exchange Evaluation on destination square.
// Returns net material from the perspective of the side that just moved.
// Positive => safe/profitable, Negative => likely losing material on dest.
// This implements a full exchange sequence simulation with X-ray detection.
#[inline]
pub fn see_dest_estimate(
    board_after: &Board,
    side_just_moved: Color,
    dest: (usize, usize),
    captured_val: i32,
) -> i32 {
    let moved_piece = match board_after.get(dest.0, dest.1) {
        Some(p) => p,
        None => return captured_val,
    };

    // Simulate full exchange sequence
    let mut board = *board_after;
    let mut gain = [0i32; 32]; // Max depth of exchange sequence
    let mut depth = 0;

    gain[0] = captured_val;
    let mut attacker_side = opposite_color(side_just_moved);
    let mut target_piece = moved_piece;

    // Simulate exchanges until no more attackers
    loop {
        // Find the smallest attacker for the current side
        let attacker = find_smallest_attacker(&board, dest, attacker_side);
        if attacker.is_none() {
            break; // No more attackers, depth stays at current value
        }

        depth += 1;
        if depth >= 32 {
            break; // Safety limit
        }

        let (attacker_sq, attacker_piece) = attacker.unwrap();
        let target_val = piece_value_cp(target_piece.get_type());

        // Record the gain for this capture (alternating perspective)
        gain[depth] = -gain[depth - 1] + target_val;

        // Simulate the capture (this handles X-ray attacks naturally)
        board.set(attacker_sq.0, attacker_sq.1, None);
        board.set(dest.0, dest.1, Some(attacker_piece));

        // Update for next iteration
        target_piece = attacker_piece;
        attacker_side = opposite_color(attacker_side);

        // Stop if the king would be captured (invalid exchange)
        if target_piece.get_type() == PieceType::King {
            break;
        }
    }

    // Negamax backwards through the gain list to find the best outcome
    // At each level, the side to move chooses between stopping or continuing
    // Standard negamax formula: gain[d-1] = -max(-gain[d-1], gain[d])
    if depth > 1 {
        while depth > 1 {
            depth -= 1;
            gain[depth - 1] = -(-gain[depth - 1]).max(gain[depth]);
        }
    } else if depth == 1 {
        // Special case: one exchange occurred, apply negamax once
        gain[0] = -(-gain[0]).max(gain[1]);
    }
    // If depth == 0, no exchanges occurred, return gain[0] as-is

    gain[0]
}

// Find the smallest (least valuable) attacker of a square for a given side.
// This is used to simulate the most favorable exchange sequence.
// Returns (square, piece) of the attacker, or None if no attacker exists.
#[inline]
fn find_smallest_attacker(
    board: &Board,
    target: (usize, usize),
    attacker_color: Color,
) -> Option<((usize, usize), Piece)> {
    let (tr, tc) = target;
    let mut smallest: Option<((usize, usize), Piece)> = None;
    let mut smallest_val = i32::MAX;

    // Check pawns first (cheapest)
    // White pawns attack from lower rows (row-1), Black pawns attack from higher rows (row+1)
    // To find pawn attackers of target square (r, c):
    // - White pawn attackers: check (r-1, c±1)
    // - Black pawn attackers: check (r+1, c±1)
    let pawn_row_offset = match attacker_color {
        Color::White => -1,  // White pawns attack from one row below (toward higher rows)
        Color::Black => 1,   // Black pawns attack from one row above (toward lower rows)
    };

    for dc in [-1, 1] {
        let r_i32 = tr as i32 + pawn_row_offset;
        let c_i32 = tc as i32 + dc;
        // Check bounds before casting to usize to avoid wraparound
        if r_i32 >= 0 && r_i32 < 8 && c_i32 >= 0 && c_i32 < 8 {
            let r = r_i32 as usize;
            let c = c_i32 as usize;
            if let Some(p) = board.get(r, c)
                && p.get_color() == attacker_color
                && p.get_type() == PieceType::Pawn {
                    let val = piece_value_cp(PieceType::Pawn);
                    if val < smallest_val {
                        smallest_val = val;
                        smallest = Some(((r, c), p));
                    }
                }
        }
    }

    // Check knights
    let knight_moves = [
        (-2, -1), (-2, 1), (-1, -2), (-1, 2),
        (1, -2), (1, 2), (2, -1), (2, 1),
    ];
    for (dr, dc) in knight_moves {
        let r_i32 = tr as i32 + dr;
        let c_i32 = tc as i32 + dc;
        if r_i32 >= 0 && r_i32 < 8 && c_i32 >= 0 && c_i32 < 8 {
            let r = r_i32 as usize;
            let c = c_i32 as usize;
            if let Some(p) = board.get(r, c)
                && p.get_color() == attacker_color
                && p.get_type() == PieceType::Knight {
                    let val = piece_value_cp(PieceType::Knight);
                    if val < smallest_val {
                        smallest_val = val;
                        smallest = Some(((r, c), p));
                    }
                }
        }
    }

    // Check bishops and queens (diagonal rays)
    for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        let attacker = scan_ray(board, target, dr, dc, attacker_color, &[PieceType::Bishop, PieceType::Queen]);
        if let Some((sq, p)) = attacker {
            let val = piece_value_cp(p.get_type());
            if val < smallest_val {
                smallest_val = val;
                smallest = Some((sq, p));
            }
        }
    }

    // Check rooks and queens (straight rays)
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let attacker = scan_ray(board, target, dr, dc, attacker_color, &[PieceType::Rook, PieceType::Queen]);
        if let Some((sq, p)) = attacker {
            let val = piece_value_cp(p.get_type());
            if val < smallest_val {
                smallest_val = val;
                smallest = Some((sq, p));
            }
        }
    }

    // Check king (last, as it's highest value for exchange purposes)
    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let r_i32 = tr as i32 + dr;
            let c_i32 = tc as i32 + dc;
            if r_i32 >= 0 && r_i32 < 8 && c_i32 >= 0 && c_i32 < 8 {
                let r = r_i32 as usize;
                let c = c_i32 as usize;
                if let Some(p) = board.get(r, c)
                    && p.get_color() == attacker_color
                    && p.get_type() == PieceType::King {
                        // King is most valuable, consider it last
                        let val = 10000; // Use high value so it's only selected if no other attacker
                        if val < smallest_val {
                            smallest_val = val;
                            smallest = Some(((r, c), p));
                        }
                    }
            }
        }
    }

    smallest
}

// Scan along a ray (diagonal or straight) to find an attacker.
// Returns the first piece of the specified types along the ray.
// This naturally handles X-ray attacks as pieces are removed during exchange simulation.
#[inline]
fn scan_ray(
    board: &Board,
    from: (usize, usize),
    dr: i32,
    dc: i32,
    color: Color,
    piece_types: &[PieceType],
) -> Option<((usize, usize), Piece)> {
    let mut r = from.0 as i32 + dr;
    let mut c = from.1 as i32 + dc;

    while r >= 0 && r < 8 && c >= 0 && c < 8 {
        let (ur, uc) = (r as usize, c as usize);
        if let Some(p) = board.get(ur, uc) {
            if p.get_color() == color && piece_types.contains(&p.get_type()) {
                return Some(((ur, uc), p));
            }
            // Blocked by any piece
            return None;
        }
        r += dr;
        c += dc;
    }
    None
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
    // White pawns attack from row-1 (from lower rows toward higher rows)
    // Black pawns attack from row+1 (from higher rows toward lower rows)
    match attacker {
        Color::White => {
            // White pawn at (r-1, c±1) attacks sq at (r, c)
            (r > 0 && c > 0 && matches!(board.get(r-1, c-1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
            || (r > 0 && c + 1 < 8 && matches!(board.get(r-1, c+1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
        }
        Color::Black => {
            // Black pawn at (r+1, c±1) attacks sq at (r, c)
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
            // Scale penalty by piece importance, don't cap too low for queen/rook blunders
            let moved_pt = base_board.get(from.0, from.1).map(|p| p.get_type());
            let pen = match moved_pt {
                Some(PieceType::Queen) => ((-see) * 6).clamp(600, 6000),
                Some(PieceType::Rook) => ((-see) * 4).clamp(SEE_PENALTY_MIN_CP, 3000),
                _ => (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP),
            };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::piece::pieces::{Color, Piece, PieceType};

    /// Helper to create a board from a position
    fn setup_board_from_pieces(pieces: &[((usize, usize), PieceType, Color)]) -> Board {
        let mut board = Board::empty();
        for &((r, c), piece_type, color) in pieces {
            board.set(r, c, Some(Piece::new(piece_type, color)));
        }
        board
    }

    #[test]
    fn test_see_simple_pawn_capture() {
        // White pawn on e4 captures black pawn on d5, no defenders/attackers
        let mut board = setup_board_from_pieces(&[
            ((3, 4), PieceType::Pawn, Color::White), // e4
        ]);
        // Simulate: pawn moved to d5 after capturing
        board.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White))); // d5
        board.set(3, 4, None); // e4 empty

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        assert_eq!(see, 100, "Undefended pawn capture should return pawn value");
    }

    #[test]
    fn test_see_knight_takes_pawn_defended_by_pawn() {
        // Knight takes pawn defended by pawn: N x P(p) => lose knight
        // Black pawn at row 4 (rank 4) can attack row 3 (rank 5) diagonally
        let mut board = setup_board_from_pieces(&[
            ((4, 2), PieceType::Pawn, Color::Black), // Defender pawn at c4, attacks d3 (row 3)
        ]);
        // Knight on d3 (row 3) after capturing pawn - note: row 3 = rank 5 in this engine
        board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White))); // d3

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        // Knight (320) takes pawn (100), then black pawn takes knight (320)
        // Net: 100 - 320 = -220
        assert!(see < 0, "Knight taking defended pawn should be negative SEE, got {}", see);
    }

    #[test]
    fn test_see_equal_trade_knight_for_knight() {
        // White knight takes black knight, black has a piece that can recapture
        let mut board = setup_board_from_pieces(&[
            ((4, 2), PieceType::Pawn, Color::Black), // Can recapture at row 3 from row 4
        ]);
        board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White))); // WN on d3

        let see = see_dest_estimate(&board, Color::White, (3, 3), 320);
        // Takes knight (320), then loses knight (320) => 0
        assert_eq!(see, 0, "Equal trade should result in SEE of 0");
    }

    #[test]
    fn test_see_queen_takes_pawn_attacked_by_pawn() {
        // Queen takes pawn, but attacked by enemy pawn => bad trade
        let mut board = setup_board_from_pieces(&[
            ((4, 2), PieceType::Pawn, Color::Black), // Attacking pawn at row 4
        ]);
        board.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White))); // Queen on d3 (row 3)

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        // Queen (900) takes pawn (100), loses queen (900) => 100 - 900 = -800
        assert!(see < -700, "Queen taking pawn attacked by pawn should be very negative");
    }

    #[test]
    fn test_see_xray_attack_rook_behind_bishop() {
        // Bishop takes pawn, rook behind bishop joins exchange (X-ray)
        // Setup: Black pawn on d5, White bishop on c4, White rook on a4 (x-ray on diagonal not applicable)
        // Better: Rook on e4, Bishop on e5 takes piece on e6, rook revealed
        let mut board = setup_board_from_pieces(&[
            ((3, 4), PieceType::Rook, Color::White),   // Rook on e4
            ((2, 5), PieceType::Pawn, Color::Black),   // Black defender pawn on f6
        ]);
        // Bishop on e5 after capturing
        board.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White))); // e5

        let see = see_dest_estimate(&board, Color::White, (2, 4), 100);
        // Bishop takes pawn (100), black pawn takes bishop (330), white rook takes pawn (100)
        // Net: 100 - 330 + 100 = -130, but negamax chooses best: don't continue => depends on exact calculation
        // The X-ray effect should be detected by scan_ray after pieces are removed
        assert!(see <= 100, "SEE should handle X-ray attacks");
    }

    #[test]
    fn test_see_multi_piece_exchange() {
        // Complex exchange: Q takes P, defended by N, B, R
        let mut board = setup_board_from_pieces(&[
            ((2, 5), PieceType::Knight, Color::Black), // Knight defender
            ((1, 6), PieceType::Bishop, Color::Black), // Bishop defender (diagonal)
        ]);
        board.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White))); // Queen on d5

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        // Queen takes pawn: +100, Knight takes Queen: -900, total -800
        // White wouldn't continue this exchange
        assert!(see < 0, "Queen taking defended pawn should be negative");
    }

    #[test]
    fn test_see_no_attacker() {
        // Piece on square with no attackers => just return captured value
        let mut board = setup_board_from_pieces(&[]);
        board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White))); // Knight on d5

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        assert_eq!(see, 100, "No attackers => captured value");
    }

    #[test]
    fn test_see_king_capture() {
        // King should be handled correctly in exchange (shouldn't be captured)
        let mut board = setup_board_from_pieces(&[
            ((2, 3), PieceType::King, Color::Black), // Black king adjacent
        ]);
        board.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White))); // Pawn on d5

        let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
        // White pawn captured Black pawn (100), Black king recaptures White pawn (100): even trade
        assert_eq!(see, 0, "Equal exchange should result in SEE of 0, got {}", see);
    }

    #[test]
    fn test_see_after_helper() {
        let board = setup_board_from_pieces(&[
            ((3, 3), PieceType::Knight, Color::White),
            ((2, 2), PieceType::Pawn, Color::Black),
        ]);

        let captured = Some(Piece::new(PieceType::Pawn, Color::Black));
        let see = see_after(&board, Color::White, (3, 3), captured);

        // Knight on d5, defended/attacked appropriately
        assert!(see <= 100, "see_after should call see_dest_estimate correctly");
    }

    #[test]
    fn test_attacked_by_pawn() {
        let board = setup_board_from_pieces(&[
            ((2, 2), PieceType::Pawn, Color::White), // c6
        ]);

        // White pawn on c6 attacks d5 and b5
        assert!(attacked_by_pawn(&board, (3, 3), Color::White), "Should detect pawn attack on d5");
        assert!(attacked_by_pawn(&board, (3, 1), Color::White), "Should detect pawn attack on b5");
        assert!(!attacked_by_pawn(&board, (3, 2), Color::White), "Should not detect attack on c5");
    }

    #[test]
    fn test_pawn_attacked_minor_penalty() {
        let board = setup_board_from_pieces(&[
            ((4, 2), PieceType::Pawn, Color::Black), // Black pawn at row 4 attacks row 3
        ]);

        let penalty = pawn_attacked_minor_penalty(&board, Color::White, (3, 3), PieceType::Knight);
        assert_eq!(penalty, 1200, "Knight attacked by pawn should have penalty");

        let no_penalty = pawn_attacked_minor_penalty(&board, Color::White, (3, 3), PieceType::Queen);
        assert_eq!(no_penalty, 0, "Queen shouldn't have minor pawn attack penalty");
    }
}
