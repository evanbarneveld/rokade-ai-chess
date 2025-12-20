use regex::Regex;
use crate::board::Board;
use crate::parser::san_move_resolver::{ResolvedSanMove, SanMoveResolver};
use crate::piece::pieces::Color;

pub struct ParsedMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<char>
}

#[derive(Debug)]
pub struct MoveParser {
    san_move_resolver: SanMoveResolver
}

impl MoveParser {
    pub fn new() -> Self {
        MoveParser {
            san_move_resolver: SanMoveResolver {}
        }
    }

    pub fn parse(&mut self, board: &Board, active_color:Color, move_san: &str) -> Result<ParsedMove, String> {

        let conversion_result = self.convert_and_validate_san_move(board, active_color, move_san);

        if conversion_result.is_err() { return Err(conversion_result.err().unwrap()); }

        let standard_move = conversion_result.unwrap();

        let from_coord_str2 = &standard_move.resolved_san_move[0..2];
        let to_coord_str2 = &standard_move.resolved_san_move[2..4];

        fn parse_sq(sq: &str) -> (usize, usize) {
            let bytes = sq.as_bytes();
            let file = (bytes[0] as char).to_ascii_lowercase();
            let rank = bytes[1] as char;

            let col = (file as u8 - b'a') as usize; // a->0, h->7
            let row = (rank as u8 - b'1') as usize; // 1->0, 8->7
            (row, col)
        }

        let from = parse_sq(from_coord_str2);
        let to   = parse_sq(to_coord_str2);

        Ok(ParsedMove {
            from,
            to,
            is_capture:standard_move.is_capture,
            is_king_side_castle:standard_move.is_king_side_castle,
            is_queen_side_castle:standard_move.is_queen_side_castle,
            promotion_piece:standard_move.promotion_piece
        })
    }

    fn convert_and_validate_san_move(&mut self, board: &Board, active_color: Color, san_move: &str) -> Result<ResolvedSanMove, String> {
        let re = Regex::new(r"^([NBRQK])?([a-h])?([1-8])?(x)?([a-h][1-8])(=[NBRQK])?(\+|#)?$|^(O-O-O)?$|^(O-O)?$").unwrap();
        let caps: Vec<_> = re.captures_iter(san_move).collect();

        //println!("{:?}", caps);

        if caps.len() == 0 || caps.get(0).is_none() {
            return Err(String::from("Invalid move format"));
        }

        let cap0 = caps.get(0).unwrap();
        if cap0.get(0).map(|m| m.as_str()) != Some(san_move) {
            return Err(String::from("Invalid move format"));
        }

        let is_capture = cap0.get(4).is_some();
        let is_promotion = cap0.get(6).is_some();

        let is_queen_side_castle = cap0.get(0).map(|m| m.as_str()) == Some("O-O-O");
        let is_king_side_castle = cap0.get(0).map(|m| m.as_str()) == Some("O-O");

        let piece_char = cap0.get(1).map(|m| m.as_str()).unwrap_or("P").chars().nth(0).unwrap();

        let promotion_piece: Option<char> = if is_promotion {
            cap0.get(6).map(|m|m.as_str().chars().nth(1)).unwrap()
        } else {
            None
        };

        //println!("{:?}", promotion_piece);

        let move_from = format!("{}{}",
                               cap0.get(2).map(|m| m.as_str()).unwrap_or("?"),
                               cap0.get(3).map(|m| m.as_str()).unwrap_or("?")
        );

        let move_to = format!("{}", cap0.get(5).map(|m| m.as_str()).unwrap_or("?"));

        println!("Piece: {}, Move from: {}, move to: {}, capture: {}, promotion: {}, king side castle: {}, queen side castle: {}", piece_char, move_from, move_to, is_capture, is_promotion, is_king_side_castle, is_queen_side_castle);

        if !is_king_side_castle && !is_queen_side_castle && move_from.contains("?") {
            self.san_move_resolver.resolve_san_move(piece_char, move_from.as_str(), move_to.as_str(), is_capture, promotion_piece, board, active_color)
        } else {
            let san_move = format!("{}{}", move_from, move_to);
            Ok(ResolvedSanMove {
               resolved_san_move:san_move,
               is_capture,
               is_king_side_castle,
               is_queen_side_castle,
               promotion_piece
            })
        }
    }

}