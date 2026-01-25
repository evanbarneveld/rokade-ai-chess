use regex::Regex;
use crate::board::Board;
use crate::piece::as_square_str;
use crate::piece::pieces::Color;
use crate::piece::pieces::{Piece, PieceType};
use crate::search::core::advanced_search::find_all_valid_moves;
use crate::state::game_state::GameState;

pub struct ParsedMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<Piece>
}

#[derive(Debug)]
pub struct MoveParser {
}

pub struct CompletedSanMove {
    pub resolved_san_move: String,
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<Piece>
}

impl Default for MoveParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveParser {
    pub fn new() -> Self {
        MoveParser {
        }
    }

    pub fn parse(&mut self, board: &mut Board, active_color:Color, move_san: &str, en_passant_target: Option<(usize,usize)>) -> Result<ParsedMove, String> {

        let conversion_result = self.convert_and_validate_san_move(board, active_color, move_san, en_passant_target);

        if conversion_result.is_err() { return Err(conversion_result.err().unwrap()); }

        let standard_move = conversion_result.unwrap();

        let from_coord_str2 = &standard_move.resolved_san_move[0..2];
        let to_coord_str2 = &standard_move.resolved_san_move[2..4];

        let from = parse_sq(from_coord_str2)?;
        let to   = parse_sq(to_coord_str2)?;

        Ok(ParsedMove {
            from,
            to,
            is_capture:standard_move.is_capture,
            is_king_side_castle:standard_move.is_king_side_castle,
            is_queen_side_castle:standard_move.is_queen_side_castle,
            promotion_piece:standard_move.promotion_piece
        })
    }

    fn convert_and_validate_san_move(&mut self, board: &mut Board, active_color: Color, san_move: &str, en_passant_target:Option<(usize,usize)>) -> Result<CompletedSanMove, String> {
        let re = Regex::new(r"^([NBRQK])?([a-h])?([1-8])?(x)?([a-h][1-8])(=[NBRQK])?([+#])?$|^(O-O-O)?([+#])?$|^(O-O)?([+#])?$").unwrap();
        let caps: Vec<_> = re.captures_iter(san_move).collect();

        //println!("{:?}", caps);

        if caps.is_empty() || caps.is_empty() {
            return Err(String::from("Invalid move format"));
        }

        let cap0 = caps.first().unwrap();
        if cap0.get(0).map(|m| m.as_str()) != Some(san_move) {
            return Err(String::from("Invalid move format"));
        }

        let is_capture = cap0.get(4).is_some();
        let is_promotion = cap0.get(6).is_some();

        let all_matches = cap0.get(0);
        let all_matched_as_str = all_matches.unwrap().as_str();

        let is_queen_side_castle = all_matched_as_str.starts_with("O-O-O");
        let is_king_side_castle = !is_queen_side_castle && all_matched_as_str.starts_with("O-O");

        let move_piece_char = cap0.get(1).map(|m| m.as_str()).unwrap_or("P").chars().nth(0).unwrap();

        let promotion_piece: Option<Piece> = if is_promotion {
            let mut promotion_piece_char = cap0.get(6).map(|m|m.as_str().chars().nth(1)).unwrap().unwrap();
            if active_color == Color::White {
                promotion_piece_char = promotion_piece_char.to_ascii_uppercase();
            } else {
                promotion_piece_char = promotion_piece_char.to_ascii_lowercase();
            }
            Piece::from_fen_char(promotion_piece_char)
        } else {
            None
        };

        //println!("{:?}", promotion_piece);

        let move_to = cap0.get(5).map(|m| m.as_str()).unwrap_or("?").to_string();

        //println!("Piece: {}, Move from: {}, move to: {}, capture: {}, promotion: {}, king side castle: {}, queen side castle: {}", piece_char, move_from, move_to, is_capture, is_promotion, is_king_side_castle, is_queen_side_castle);

        if is_king_side_castle || is_queen_side_castle {
            // Resolve castling into explicit from/to squares based on active color
            let (from_sq, to_sq) = match active_color {
                Color::White => {
                    if is_king_side_castle { ("e1", "g1") } else { ("e1", "c1") }
                }
                Color::Black => {
                    if is_king_side_castle { ("e8", "g8") } else { ("e8", "c8") }
                }
            };
            let san_move = format!("{}{}", from_sq, to_sq);
            Ok(CompletedSanMove {
               resolved_san_move:san_move,
               is_capture,
               is_king_side_castle,
               is_queen_side_castle,
               promotion_piece
            })
        } else {
            let from_file = cap0.get(2).map(|m| m.as_str().chars().next().unwrap());
            let from_rank = cap0.get(3).map(|m| m.as_str().chars().next().unwrap());
            self.resolve_with_legal_moves(
                board,
                active_color,
                move_piece_char,
                from_file,
                from_rank,
                move_to.as_str(),
                is_capture,
                promotion_piece,
                en_passant_target,
            )
        }
    }

    fn resolve_with_legal_moves(
        &self,
        board: &mut Board,
        active_color: Color,
        piece_char: char,
        from_file: Option<char>,
        from_rank: Option<char>,
        move_to: &str,
        is_capture: bool,
        promotion_piece: Option<Piece>,
        en_passant_target: Option<(usize, usize)>,
    ) -> Result<CompletedSanMove, String> {
        let to = parse_sq(move_to)?;
        let piece_type = piece_char_to_type(piece_char)?;

        let mut gs = GameState::from_board_and_side(*board, active_color);
        gs.set_en_passant_target(en_passant_target);

        let moves = find_all_valid_moves(&mut gs);
        let mut candidates: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();

        for (from, to_mv, promo) in moves {
            if to_mv != to {
                continue;
            }

            let moving_piece = match gs.board().get(from.0, from.1) {
                Some(p) => p,
                None => continue,
            };
            if moving_piece.get_type() != piece_type {
                continue;
            }

            if let Some(f) = from_file {
                let expected = (f as u8 - b'a') as usize;
                if from.1 != expected {
                    continue;
                }
            }
            if let Some(r) = from_rank {
                let expected = (r as u8 - b'1') as usize;
                if from.0 != expected {
                    continue;
                }
            }

            let is_target_occupied = gs.board().get(to.0, to.1).is_some();
            let is_ep = moving_piece.get_type() == PieceType::Pawn
                && en_passant_target.is_some()
                && to == en_passant_target.unwrap()
                && !is_target_occupied
                && from.1 != to.1;
            let is_capture_actual = is_target_occupied || is_ep;
            if is_capture != is_capture_actual {
                continue;
            }

            if let Some(promo_piece) = promotion_piece {
                let promo_type = promo_piece.get_type();
                let promo_char = promo.map(|c| c.to_ascii_lowercase());
                if promo_char != piece_type_to_promo_char(promo_type) {
                    continue;
                }
            } else {
                if candidates.iter().any(|(f, t, _)| *f == from && *t == to_mv) {
                    continue;
                }
            }

            candidates.push((from, to_mv, promo));
        }

        if candidates.len() != 1 {
            return Err(String::from("Invalid move"));
        }

        let (from, to, _) = candidates[0];
        let resolved_move = format!("{}{}", as_square_str(from), as_square_str(to));
        Ok(CompletedSanMove {
            resolved_san_move: resolved_move,
            is_capture,
            is_king_side_castle: false,
            is_queen_side_castle: false,
            promotion_piece,
        })
    }
}

fn piece_char_to_type(piece: char) -> Result<PieceType, String> {
    match piece {
        'P' => Ok(PieceType::Pawn),
        'N' => Ok(PieceType::Knight),
        'B' => Ok(PieceType::Bishop),
        'R' => Ok(PieceType::Rook),
        'Q' => Ok(PieceType::Queen),
        'K' => Ok(PieceType::King),
        _ => Err(String::from("Invalid piece type")),
    }
}

fn piece_type_to_promo_char(pt: PieceType) -> Option<char> {
    match pt {
        PieceType::Queen => Some('q'),
        PieceType::Rook => Some('r'),
        PieceType::Bishop => Some('b'),
        PieceType::Knight => Some('n'),
        _ => None,
    }
}

fn parse_sq(sq: &str) -> Result<(usize, usize), String> {
    let bytes = sq.as_bytes();
    if bytes.len() != 2 {
        return Err(String::from("Invalid square"));
    }
    let file = (bytes[0] as char).to_ascii_lowercase();
    let rank = bytes[1] as char;

    if !(b'a'..=b'h').contains(&(file as u8)) || !(b'1'..=b'8').contains(&(rank as u8)) {
        return Err(String::from("Invalid square"));
    }

    let col = (file as u8 - b'a') as usize; // a->0, h->7
    let row = (rank as u8 - b'1') as usize; // 1->0, 8->7
    Ok((row, col))
}
