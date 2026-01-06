
#[derive(Debug, Clone, Copy)]
pub struct CastlingRights {
    pub(crate) white_kingside: bool,
    pub(crate) white_queenside: bool,
    pub(crate) black_kingside: bool,
    pub(crate) black_queenside: bool,
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

    // Mutators
    pub fn revoke_white_castling(&mut self) {
        self.white_kingside = false;
        self.white_queenside = false;
    }

    pub fn revoke_black_castling(&mut self) {
        self.black_kingside = false;
        self.black_queenside = false;
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

    // Accessors used by move validators
    pub fn white_kingside(&self) -> bool { self.white_kingside }
    pub fn white_queenside(&self) -> bool { self.white_queenside }
    pub fn black_kingside(&self) -> bool { self.black_kingside }
    pub fn black_queenside(&self) -> bool { self.black_queenside }
}