#[derive(Debug, Clone)]
pub struct CastlingRights {
    white_kingside: bool,
    white_queenside: bool,
    black_kingside: bool,
    black_queenside: bool,
}

impl CastlingRights {
    pub fn all() -> Self {
        CastlingRights {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    }

    pub fn none() -> Self {
        CastlingRights {
            white_kingside: false,
            white_queenside: false,
            black_kingside: false,
            black_queenside: false,
        }
    }

    pub fn from_fen(s: &str) -> Self {
        if s == "-" {
            return Self::none();
        }

        CastlingRights {
            white_kingside: s.contains('K'),
            white_queenside: s.contains('Q'),
            black_kingside: s.contains('k'),
            black_queenside: s.contains('q'),
        }
    }

    pub fn to_fen(&self) -> String {
        let mut result = String::new();
        if self.white_kingside { result.push('K'); }
        if self.white_queenside { result.push('Q'); }
        if self.black_kingside { result.push('k'); }
        if self.black_queenside { result.push('q'); }
        if result.is_empty() { result.push('-'); }
        result
    }
}