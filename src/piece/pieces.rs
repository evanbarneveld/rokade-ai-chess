#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    White,
    Black,
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
            PieceType::King => '♚',
            PieceType::Queen => '♛',
            PieceType::Rook => '♜',
            PieceType::Bishop => '♝',
            PieceType::Knight => '♞',
            PieceType::Pawn => '♟',
        }
    }

    pub fn from_fen_char(c: char) -> Option<Piece> {
        match c {
            'K' => Some(Piece::new(PieceType::King, Color::White)),
            'Q' => Some(Piece::new(PieceType::Queen, Color::White)),
            'R' => Some(Piece::new(PieceType::Rook, Color::White)),
            'B' => Some(Piece::new(PieceType::Bishop, Color::White)),
            'N' => Some(Piece::new(PieceType::Knight, Color::White)),
            'P' => Some(Piece::new(PieceType::Pawn, Color::White)),
            'k' => Some(Piece::new(PieceType::King, Color::Black)),
            'q' => Some(Piece::new(PieceType::Queen, Color::Black)),
            'r' => Some(Piece::new(PieceType::Rook, Color::Black)),
            'b' => Some(Piece::new(PieceType::Bishop, Color::Black)),
            'n' => Some(Piece::new(PieceType::Knight, Color::Black)),
            'p' => Some(Piece::new(PieceType::Pawn, Color::Black)),
            _ => None,
        }
    }

    pub fn to_fen_char(&self) -> char {
        match (self.color, self.piece_type) {
            (Color::White, PieceType::King) => 'K',
            (Color::White, PieceType::Queen) => 'Q',
            (Color::White, PieceType::Rook) => 'R',
            (Color::White, PieceType::Bishop) => 'B',
            (Color::White, PieceType::Knight) => 'N',
            (Color::White, PieceType::Pawn) => 'P',
            (Color::Black, PieceType::King) => 'k',
            (Color::Black, PieceType::Queen) => 'q',
            (Color::Black, PieceType::Rook) => 'r',
            (Color::Black, PieceType::Bishop) => 'b',
            (Color::Black, PieceType::Knight) => 'n',
            (Color::Black, PieceType::Pawn) => 'p',
        }
    }
}
