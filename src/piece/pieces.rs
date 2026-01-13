use crate::piece::pieces::PieceType::{Bishop, King, Knight, Pawn, Queen, Rook};


pub const PAWN_VALUE: i32 = 100;
pub const KNIGHT_VALUE: i32 = 320;
pub const BISHOP_VALUE: i32 = 330;
pub const ROOK_VALUE: i32 = 500;
pub const QUEEN_VALUE: i32 = 900;
pub const KING_VALUE: i32 = 20_000;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[inline]
pub fn piece_value_cp(pt: PieceType) -> i32 {
    use crate::piece::pieces::PieceType::*;
    match pt {
        Pawn => PAWN_VALUE,
        Knight => KNIGHT_VALUE,
        Bishop => BISHOP_VALUE,
        Rook => ROOK_VALUE,
        Queen => QUEEN_VALUE,
        King => KING_VALUE,
    }
}

#[inline]
pub fn capture_value_cp(pt: PieceType) -> i32 {
    match pt {
        Pawn => PAWN_VALUE,
        Knight => KNIGHT_VALUE,
        Bishop => BISHOP_VALUE,
        Rook => ROOK_VALUE,
        Queen => QUEEN_VALUE,
        King => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

#[inline]
pub fn opposite_color(c: Color) -> Color {
    match c {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Piece {
    piece_type: PieceType,
    color: Color,
}

impl Piece {
    pub fn new(piece_type: PieceType, color: Color) -> Self {
        Piece { piece_type, color }
    }

    pub fn get_type(&self) -> PieceType {
        self.piece_type
    }
    
    pub fn get_color(&self) -> Color {
        self.color
    }
    
    pub fn symbol(&self) -> char {
        // Use the white piece Unicode characters for both colors; colorization is handled in Board display
        match self.piece_type {
            King => '♚',
            Queen => '♛',
            Rook => '♜',
            Bishop => '♝',
            Knight => '♞',
            Pawn => '\u{2659}', // works well but is not filled in
            // Pawn => '\u{1FA05}', // doesn't work
            //Pawn => '♟', // works not it terminal: pawn gets purple color!
            //Pawn => '\u{265F}', // works not it terminal: pawn gets purple color!
        }
    }

    pub fn from_fen_char(c: char) -> Option<Piece> {
        match c {
            'K' => Some(Piece::new(King, Color::White)),
            'Q' => Some(Piece::new(Queen, Color::White)),
            'R' => Some(Piece::new(Rook, Color::White)),
            'B' => Some(Piece::new(Bishop, Color::White)),
            'N' => Some(Piece::new(Knight, Color::White)),
            'P' => Some(Piece::new(Pawn, Color::White)),
            'k' => Some(Piece::new(King, Color::Black)),
            'q' => Some(Piece::new(Queen, Color::Black)),
            'r' => Some(Piece::new(Rook, Color::Black)),
            'b' => Some(Piece::new(Bishop, Color::Black)),
            'n' => Some(Piece::new(Knight, Color::Black)),
            'p' => Some(Piece::new(Pawn, Color::Black)),
            _ => None,
        }
    }

    pub fn to_fen_char(&self) -> char {
        match (self.color, self.piece_type) {
            (Color::White, King) => 'K',
            (Color::White, Queen) => 'Q',
            (Color::White, Rook) => 'R',
            (Color::White, Bishop) => 'B',
            (Color::White, Knight) => 'N',
            (Color::White, Pawn) => 'P',
            (Color::Black, King) => 'k',
            (Color::Black, Queen) => 'q',
            (Color::Black, Rook) => 'r',
            (Color::Black, Bishop) => 'b',
            (Color::Black, Knight) => 'n',
            (Color::Black, Pawn) => 'p',
        }
    }
}
