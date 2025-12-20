use crate::piece::pieces::Color;
use crate::state::game_state::GameState;

pub fn game_state_to_fen_string(game_state: GameState) -> String {
    let mut fen = String::new();

    // Field 1: Piece placement
    for row in (0..8).rev() {
        let mut empty_count = 0;

        for col in 0..8 {
            if let Some(piece) = game_state.board().get(row, col) {
                if empty_count > 0 {
                    fen.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                fen.push(piece.to_fen_char());
            } else {
                empty_count += 1;
            }
        }

        if empty_count > 0 {
            fen.push_str(&empty_count.to_string());
        }

        if row > 0 {
            fen.push('/');
        }
    }

    // Field 2: Active color
    fen.push(' ');
    fen.push(match game_state.active_color() {
        Color::White => 'w',
        Color::Black => 'b',
    });

    // Field 3: Castling rights
    fen.push(' ');
    fen.push_str(&game_state.castling_rights().to_fen());

    // Field 4: En passant target
    fen.push(' ');
    if let Some((row, col)) = game_state.en_passant_target() {
        fen.push((b'a' + col as u8) as char);
        fen.push_str(&(row + 1).to_string());
    } else {
        fen.push('-');
    }

    // Field 5: Halfmove clock
    fen.push(' ');
    fen.push_str(&game_state.half_move_clock().to_string());

    // Field 6: Fullmove number
    fen.push(' ');
    fen.push_str(&game_state.full_move_number().to_string());

    fen
}