use crate::piece::piece_mover::PieceMover;
use crate::search::advanced_search::find_all_valid_moves;
use crate::state::game_state::GameState;

#[inline]
pub fn perft_count(gs: &GameState, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = find_all_valid_moves(gs);

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes: u64 = 0;
    for (from, to, promo) in moves {
        let mut child = *gs; // GameState is Copy
        // capture may be en passant even if target square is empty
        let mover = child.board().get(from.0, from.1);
        let is_capture = child.board().get(to.0, to.1).is_some()
            || (mover.is_some()
                && mover.unwrap().get_type() == crate::piece::pieces::PieceType::Pawn
                && child.en_passant_target().is_some()
                && child.en_passant_target().unwrap() == to);
        let promote_piece = match promo {
            Some('q') | Some('Q') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Queen,
                gs.active_color(),
            )),
            Some('r') | Some('R') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Rook,
                gs.active_color(),
            )),
            Some('b') | Some('B') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Bishop,
                gs.active_color(),
            )),
            Some('n') | Some('N') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Knight,
                gs.active_color(),
            )),
            _ => None,
        };
        if PieceMover::move_piece(&mut child, from, to, is_capture, promote_piece) {
            child.switch_player_turn();
            nodes += perft_count(&child, depth - 1);
        }
    }
    nodes
}

#[allow(dead_code)]
pub fn perft_divide(gs: &GameState, depth: u32) -> Vec<(String, u64)> {
    let mut result = Vec::new();
    if depth == 0 {
        return result;
    }
    let moves = find_all_valid_moves(gs);
    for (from, to, promo) in moves {
        let mut child = *gs;
        // capture may be en passant even if target square is empty
        let mover = child.board().get(from.0, from.1);
        let is_capture = child.board().get(to.0, to.1).is_some()
            || (mover.is_some()
                && mover.unwrap().get_type() == crate::piece::pieces::PieceType::Pawn
                && child.en_passant_target().is_some()
                && child.en_passant_target().unwrap() == to);
        let promote_piece = match promo {
            Some('q') | Some('Q') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Queen,
                gs.active_color(),
            )),
            Some('r') | Some('R') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Rook,
                gs.active_color(),
            )),
            Some('b') | Some('B') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Bishop,
                gs.active_color(),
            )),
            Some('n') | Some('N') => Some(crate::piece::pieces::Piece::new(
                crate::piece::pieces::PieceType::Knight,
                gs.active_color(),
            )),
            _ => None,
        };
        if PieceMover::move_piece(&mut child, from, to, is_capture, promote_piece) {
            child.switch_player_turn();
            let count = perft_count(&child, depth - 1);
            // format move as coordinate string
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
            result.push((s, count));
        }
    }
    result
}
