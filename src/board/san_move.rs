use crate::board::Board;
use crate::piece::as_square_str;
use crate::piece::pieces::{Color, PieceType};
use crate::state::game_state::GameState;

pub fn convert_move_to_san(game_state : GameState, generated_move: Option<((usize, usize), (usize, usize))>) -> Option<String> {

    if generated_move.is_none() { return None; }

    let some_generated_move = generated_move.unwrap();

    let board = game_state.board();

    let square_move = get_san_move_if_casting_move(board, some_generated_move.0, some_generated_move.1);

    if square_move.is_some() {
        return square_move;
    }

    let (prefix, is_pawn_promotion) = get_san_move_piece_prefix(board, some_generated_move.0, some_generated_move.1);
    // If this is a capture, reflect that in the SAN-like output by inserting an 'x'
    if board.get(some_generated_move.1.0, some_generated_move.1.1).is_some() {
        let pawn_promotion = if is_pawn_promotion { "=Q" } else { "" };
        return Some(format!("{}{}x{}{}", prefix, as_square_str(some_generated_move.0), as_square_str(some_generated_move.1), pawn_promotion));
    }

    // Default to simple coordinate move (e.g., e2e4), with optional prefix for piece type
    let pawn_promotion = if is_pawn_promotion { "=Q" } else { "" };
    Some(format!("{}{}{}{}", prefix, as_square_str(some_generated_move.0), as_square_str(some_generated_move.1), pawn_promotion))
}

fn get_san_move_piece_prefix(board: &Board, from: (usize, usize), to: (usize, usize)) -> (String, bool) {
    // Determine SAN piece prefix (empty for pawns)
    let mut prefix = String::new();
    let mut is_pawn_promotion = false;
    if let Some(p) = board.get(from.0, from.1) {
        prefix = match p.get_type() {
            PieceType::Pawn => String::new(),
            PieceType::Knight => String::from("N"),
            PieceType::Bishop => String::from("B"),
            PieceType::Rook => String::from("R"),
            PieceType::Queen => String::from("Q"),
            PieceType::King => String::from("K"),
        };

        // Detect pawn promotion (assume promotion to a queen)
        if p.get_type() == PieceType::Pawn {
            is_pawn_promotion =
                (p.get_color() == Color::White && to.0 == 7) ||
                    (p.get_color() == Color::Black && to.0 == 0);
        }
    }
    (prefix, is_pawn_promotion)
}

fn get_san_move_if_casting_move(board: &Board, from: (usize, usize), to: (usize, usize)) -> Option<String> {
    // Castling detection (king moves two files)
    if let Some(p) = board.get(from.0, from.1) {
        if p.get_type() == PieceType::King {
            // king side castle
            if to.1 == from.1 + 2 {
                return Some(String::from("O-O"));
            }
            // queen side castle
            if from.1 >= 2 && to.1 + 2 == from.1 {
                return Some(String::from("O-O-O"));
            }
        }
    }
    None
}
