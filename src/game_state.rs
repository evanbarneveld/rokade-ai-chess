use crate::board::Board;
use crate::castling_rights::CastlingRights;
use crate::pieces::{Piece, Color};

#[derive(Debug)]
pub struct GameState {
    board: Board,
    active_color: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<(usize, usize)>,
    half_move_clock: u32,
    full_move_number: u32,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            board: Board::new(),
            active_color: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant_target: None,
            half_move_clock: 0,
            full_move_number: 1,
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        self.board.move_piece(from, to)
    }

    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();

        if parts.len() != 6 {
            return Err(format!("Invalid FEN: expected 6 fields, got {}", parts.len()));
        }

        // Parse piece placement (field 1)
        let mut board = Board::empty();
        let ranks: Vec<&str> = parts[0].split('/').collect();

        if ranks.len() != 8 {
            return Err(format!("Invalid FEN: expected 8 ranks, got {}", ranks.len()));
        }

        for (rank_idx, rank_str) in ranks.iter().enumerate() {
            let row = 7 - rank_idx; // FEN starts from rank 8 (row 7)
            let mut col = 0;

            for c in rank_str.chars() {
                if let Some(digit) = c.to_digit(10) {
                    col += digit as usize;
                } else if let Some(piece) = Piece::from_fen_char(c) {
                    if col >= 8 {
                        return Err("Invalid FEN: too many pieces in rank".to_string());
                    }
                    board.set(row, col, Some(piece));
                    col += 1;
                } else {
                    return Err(format!("Invalid FEN: unknown character '{}'", c));
                }
            }

            if col != 8 {
                return Err(format!("Invalid FEN: rank {} has {} squares instead of 8", 8 - rank_idx, col));
            }
        }

        // Parse active color (field 2)
        let active_color = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(format!("Invalid FEN: active color must be 'w' or 'b', got '{}'", parts[1])),
        };

        // Parse castling rights (field 3)
        let castling_rights = CastlingRights::from_fen(parts[2]);

        // Parse en passant target (field 4)
        let en_passant_target = if parts[3] == "-" {
            None
        } else {
            let bytes = parts[3].as_bytes();
            if bytes.len() != 2 {
                return Err(format!("Invalid FEN: en passant square must be 2 characters, got '{}'", parts[3]));
            }
            let col = (bytes[0] as char).to_digit(18).and_then(|d| if d >= 10 { Some(d - 10) } else { None });
            let row = (bytes[1] as char).to_digit(10);

            match (col, row) {
                (Some(c), Some(r)) if c < 8 && r >= 1 && r <= 8 => Some((r as usize - 1, c as usize)),
                _ => return Err(format!("Invalid FEN: invalid en passant square '{}'", parts[3])),
            }
        };

        // Parse halfmove clock (field 5)
        let half_move_clock = parts[4].parse::<u32>()
            .map_err(|_| format!("Invalid FEN: halfmove clock must be a number, got '{}'", parts[4]))?;

        // Parse fullmove number (field 6)
        let full_move_number = parts[5].parse::<u32>()
            .map_err(|_| format!("Invalid FEN: fullmove number must be a number, got '{}'", parts[5]))?;

        if full_move_number == 0 {
            return Err("Invalid FEN: fullmove number must be at least 1".to_string());
        }

        Ok(GameState {
            board,
            active_color,
            castling_rights,
            en_passant_target,
            half_move_clock,
            full_move_number,
        })
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // Field 1: Piece placement
        for row in (0..8).rev() {
            let mut empty_count = 0;

            for col in 0..8 {
                if let Some(piece) = self.board.get(row, col) {
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
        fen.push(match self.active_color {
            Color::White => 'w',
            Color::Black => 'b',
        });

        // Field 3: Castling rights
        fen.push(' ');
        fen.push_str(&self.castling_rights.to_fen());

        // Field 4: En passant target
        fen.push(' ');
        if let Some((row, col)) = self.en_passant_target {
            fen.push((b'a' + col as u8) as char);
            fen.push_str(&(row + 1).to_string());
        } else {
            fen.push('-');
        }

        // Field 5: Halfmove clock
        fen.push(' ');
        fen.push_str(&self.half_move_clock.to_string());

        // Field 6: Fullmove number
        fen.push(' ');
        fen.push_str(&self.full_move_number.to_string());

        fen
    }
}
