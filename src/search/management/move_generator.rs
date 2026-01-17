use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;

/// Generate target squares for a piece based on its type and position.
/// Returns a vector of potential destination squares (not validated yet).
#[inline]
fn generate_piece_targets(piece_type: PieceType, from: (usize, usize), color: Color) -> Vec<(usize, usize)> {
    let (r, c) = from;
    let mut targets = Vec::with_capacity(32);

    match piece_type {
        PieceType::Knight => {
            // Knight: exactly 8 possible moves
            const KNIGHT_MOVES: [(isize, isize); 8] = [
                (2, 1), (2, -1), (-2, 1), (-2, -1),
                (1, 2), (1, -2), (-1, 2), (-1, -2)
            ];
            for (dr, dc) in KNIGHT_MOVES {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                }
            }
        }
        PieceType::King => {
            // King: up to 8 adjacent squares
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if dr == 0 && dc == 0 { continue; }
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if (0..8).contains(&nr) && (0..8).contains(&nc) {
                        targets.push((nr as usize, nc as usize));
                    }
                }
            }

            // Castling: king moves 2 squares left or right from starting position
            let start_row = if color == Color::White { 0 } else { 7 };
            let start_col = 4;
            if from == (start_row, start_col) {
                // Kingside castling (move to column 6)
                targets.push((start_row, 6));
                // Queenside castling (move to column 2)
                targets.push((start_row, 2));
            }
        }
        PieceType::Pawn => {
            // Pawn: forward moves and captures
            let forward = if color == Color::White { 1 } else { -1 };
            let start_rank = if color == Color::White { 1 } else { 6 };

            // Single push
            let nr = r as isize + forward;
            if (0..8).contains(&nr) {
                targets.push((nr as usize, c));

                // Double push from starting position
                if r == start_rank {
                    let nr2 = r as isize + 2 * forward;
                    if (0..8).contains(&nr2) {
                        targets.push((nr2 as usize, c));
                    }
                }
            }

            // Captures (diagonal)
            for dc in [-1, 1] {
                let nr = r as isize + forward;
                let nc = c as isize + dc;
                if (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                }
            }
        }
        PieceType::Rook | PieceType::Bishop | PieceType::Queen => {
            // Sliding pieces: generate rays
            let directions = match piece_type {
                PieceType::Rook => vec![(0, 1), (0, -1), (1, 0), (-1, 0)],
                PieceType::Bishop => vec![(1, 1), (1, -1), (-1, 1), (-1, -1)],
                PieceType::Queen => vec![
                    (0, 1), (0, -1), (1, 0), (-1, 0),
                    (1, 1), (1, -1), (-1, 1), (-1, -1)
                ],
                _ => vec![],
            };

            for (dr, dc) in directions {
                let mut nr = r as isize + dr;
                let mut nc = c as isize + dc;
                while (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                    nr += dr;
                    nc += dc;
                }
            }
        }
    }

    targets
}

/// Helper function to convert promotion char to PieceType
#[inline]
fn promo_char_to_piece_type(pc: char) -> PieceType {
    match pc {
        'q' => PieceType::Queen,
        'r' => PieceType::Rook,
        'b' => PieceType::Bishop,
        'n' => PieceType::Knight,
        _ => PieceType::Queen,
    }
}

/// Helper function to convert PieceType to promotion char
#[inline]
fn piece_type_to_promo_char(pt: PieceType) -> Option<char> {
    match pt {
        PieceType::Queen => Some('q'),
        PieceType::Rook => Some('r'),
        PieceType::Bishop => Some('b'),
        PieceType::Knight => Some('n'),
        _ => None,
    }
}

/// Check if a pawn move is a promotion
#[inline]
fn is_promotion(to: (usize, usize), active_color: Color) -> bool {
    (active_color == Color::White && to.0 == 7)
        || (active_color == Color::Black && to.0 == 0)
}

/// Try to make a move and check if it's legal (doesn't leave king in check).
/// Returns true if the move is legal.
/// This is a lightweight wrapper around the move + legality check to make the intent clear.
#[inline]
fn try_move_and_check_legality(
    game_state: &GameState,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
    promo_piece: Option<Piece>,
    active_color: Color,
) -> bool {
    let mut gs_var = *game_state;
    PieceMover::move_piece(&mut gs_var, from, to, is_capture, promo_piece)
        && !gs_var.mutable_board().is_side_in_check(active_color)
}

pub fn find_all_valid_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let active_color = game_state.active_color();

    let mut pieces_to_move = Vec::new();
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = game_state.board().get(r, c)
                && p.get_color() == active_color {
                    pieces_to_move.push(((r, c), p.get_type()));
                }
        }
    }

    let en_passant_target = game_state.en_passant_target();

    for (from, piece_type) in pieces_to_move {
        // Generate piece-specific target squares instead of checking all 64 squares
        let targets = generate_piece_targets(piece_type, from, active_color);

        for to in targets {
            let target_piece = game_state.board().get(to.0, to.1);
            let is_capture = target_piece.is_some()
                || (piece_type == PieceType::Pawn
                    && en_passant_target.is_some()
                    && to == en_passant_target.unwrap());

            if !game_state.move_from_and_to_validation_check(
                from,
                to,
                is_capture,
                piece_type == PieceType::Pawn,
            ) {
                continue;
            }

            let is_pawn_promotion = piece_type == PieceType::Pawn && is_promotion(to, active_color);

            if is_pawn_promotion {
                // Generate all 4 promotion options and filter by legality
                let promo_chars = ['q', 'r', 'b', 'n'];
                for &pc in promo_chars.iter() {
                    let promo_piece = Some(Piece::new(promo_char_to_piece_type(pc), active_color));
                    if try_move_and_check_legality(game_state, from, to, is_capture, promo_piece, active_color) {
                        result.push((from, to, Some(pc)));
                    }
                }
            } else {
                // Regular move - check legality
                if try_move_and_check_legality(game_state, from, to, is_capture, None, active_color) {
                    result.push((from, to, None));
                }
            }
        }
    }
    result
}

// Lightweight move for perft: includes capture flag and promotion marker.
#[derive(Clone, Copy, Debug)]
pub struct PerftMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub is_capture: bool,
    pub promo: Option<char>,
}

/// Fill `out` with all legal moves for the active side, including capture flag and promotion marker.
/// This mirrors `find_all_valid_moves` but avoids allocating a new Vec every call and returns flags.
pub fn find_all_valid_moves_into_perft(game_state: &GameState, out: &mut Vec<PerftMove>) {
    out.clear();
    let board = game_state.board();
    let active_color = game_state.active_color();

    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) { Some(p) => p, None => continue };
            if piece.get_color() != active_color { continue; }

            let from = (r, c);
            let piece_type = piece.get_type();

            // Generate piece-specific target squares instead of checking all 64 squares
            let targets = generate_piece_targets(piece_type, from, active_color);

            for to in targets {
                let target_piece_is_some = board.get(to.0, to.1).is_some();
                let is_capture = target_piece_is_some
                    || (piece_type == PieceType::Pawn
                        && game_state.en_passant_target().is_some()
                        && to == game_state.en_passant_target().unwrap());
                let is_pawn_move = piece_type == PieceType::Pawn;
                if !game_state.move_from_and_to_validation_check(
                    from, to, is_capture, is_pawn_move,
                ) { continue; }

                let is_pawn_promotion = piece_type == PieceType::Pawn && is_promotion(to, active_color);

                if is_pawn_promotion {
                    // Generate all 4 promotion options and filter by legality
                    let promo_types = [
                        PieceType::Queen,
                        PieceType::Rook,
                        PieceType::Bishop,
                        PieceType::Knight,
                    ];
                    for pt in promo_types.iter() {
                        let promo_piece = Some(Piece::new(*pt, active_color));
                        if try_move_and_check_legality(game_state, from, to, is_capture, promo_piece, active_color) {
                            out.push(PerftMove {
                                from,
                                to,
                                is_capture,
                                promo: piece_type_to_promo_char(*pt)
                            });
                        }
                    }
                } else {
                    // Regular move - check legality
                    if try_move_and_check_legality(game_state, from, to, is_capture, None, active_color) {
                        out.push(PerftMove { from, to, is_capture, promo: None });
                    }
                }
            }
        }
    }
}

/// Dump all legal moves for the given side from the current board, formatted as SAN or coordinate pairs.
/// This is intended for debugging/tests. It returns a single string with moves separated by spaces.
/// By default it uses simple coordinate notation like e2e4; when `to_san` is true and a GameState is
/// provided, it will attempt to convert to SAN.
pub fn _dump_all_valid_moves(
    game_state: &GameState,
    to_san: bool,
) {
    use crate::board::san_move::convert_move_to_san;
    let mut gs = *game_state;
    let moves = find_all_valid_moves(&mut gs);
    if moves.is_empty() {
        println!("No moves");
        return;
    }
    if to_san {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, promo) in moves {
            if let Some(s) = convert_move_to_san(*game_state, Some((from, to, promo))) {
                parts.push(s);
            } else {
                // fallback to coord if SAN conversion fails
                let s = format!(
                    "{}{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char,
                    promo.unwrap_or('\0')
                );
                parts.push(s.trim_end_matches('\0').to_string());
            }
        }
        println!("{}", parts.join(" "));
    } else {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, promo) in moves {
            let s = if let Some(pc) = promo {
                format!(
                    "{}{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char,
                    pc
                )
            } else {
                format!(
                    "{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char
                )
            };
            parts.push(s);
        }
        println!("{}", parts.join(" "));
    }
}
