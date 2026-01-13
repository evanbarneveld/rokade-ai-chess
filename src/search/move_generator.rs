use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;

pub fn find_all_valid_moves(
    game_state: &GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let board = game_state.board();
    let active_color = game_state.active_color();

    // iterate all squares and collect legal moves for the active color
    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color {
                continue;
            }

            for tr in 0..8 {
                for tc in 0..8 {
                    let from = (r, c);
                    let to = (tr, tc);
                    if from == to {
                        continue;
                    }

                    let target_piece_is_some = board.get(tr, tc).is_some();

                    // basic board-level validation (ownership, capture flags, bounds)
                    let is_capture = target_piece_is_some
                        || (piece.get_type() == PieceType::Pawn
                            && game_state.en_passant_target().is_some()
                            && to == game_state.en_passant_target().unwrap());
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !game_state.move_from_and_to_validation_check(
                        from,
                        to,
                        active_color,
                        is_capture,
                        is_pawn_move,
                        game_state.en_passant_target(),
                    ) {
                        continue;
                    }

                    // Use full GameState-aware move application to validate legality, covering:
                    // - pins/check (including en passant discovered checks)
                    // - castling rights and rook/king path clearance
                    // - en passant captures
                    // - promotions (try all promotion piece types)
                    let mut gs = *game_state;
                    let is_pawn_promotion = piece.get_type() == PieceType::Pawn
                        && ((active_color == Color::White && tr == 7)
                            || (active_color == Color::Black && tr == 0));

                    if is_pawn_promotion {
                        // Try all legal promotion pieces: Queen, Rook, Bishop, Knight
                        // Note: We push the same (from,to) four times if all are legal,
                        // so perft and generators can count distinct promotions separately.
                        let promo_types = [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ];
                        for pt in promo_types.iter() {
                            let mut gs_var = gs; // work from the same pre-move state
                            let promo_piece = Some(Piece::new(*pt, active_color));
                            if PieceMover::move_piece(&mut gs_var, from, to, is_capture, promo_piece)
                            {
                                let ch = match pt {
                                    PieceType::Queen => Some('q'),
                                    PieceType::Rook => Some('r'),
                                    PieceType::Bishop => Some('b'),
                                    PieceType::Knight => Some('n'),
                                    _ => None,
                                };
                                result.push((from, to, ch));
                            }
                        }
                    } else {
                        if PieceMover::move_piece(&mut gs, from, to, is_capture, None) {
                            result.push((from, to, None));
                        }
                    }
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

            for tr in 0..8 {
                for tc in 0..8 {
                    let from = (r, c);
                    let to = (tr, tc);
                    if from == to { continue; }

                    let target_piece_is_some = board.get(tr, tc).is_some();
                    let is_capture = target_piece_is_some
                        || (piece.get_type() == PieceType::Pawn
                            && game_state.en_passant_target().is_some()
                            && to == game_state.en_passant_target().unwrap());
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !game_state.move_from_and_to_validation_check(
                        from, to, active_color, is_capture, is_pawn_move, game_state.en_passant_target(),
                    ) { continue; }

                    let mut gs = *game_state;
                    let is_pawn_promotion = piece.get_type() == PieceType::Pawn
                        && ((active_color == Color::White && tr == 7)
                            || (active_color == Color::Black && tr == 0));

                    if is_pawn_promotion {
                        let promo_types = [
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ];
                        for pt in promo_types.iter() {
                            let mut gs_var = gs;
                            let promo_piece = Some(Piece::new(*pt, active_color));
                            if PieceMover::move_piece(&mut gs_var, from, to, is_capture, promo_piece) {
                                let ch = match pt {
                                    PieceType::Queen => Some('q'),
                                    PieceType::Rook => Some('r'),
                                    PieceType::Bishop => Some('b'),
                                    PieceType::Knight => Some('n'),
                                    _ => None,
                                };
                                out.push(PerftMove { from, to, is_capture, promo: ch });
                            }
                        }
                    } else {
                        if PieceMover::move_piece(&mut gs, from, to, is_capture, None) {
                            out.push(PerftMove { from, to, is_capture, promo: None });
                        }
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
    let moves = find_all_valid_moves(game_state);
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
        return;
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
