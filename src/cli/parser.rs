pub struct ParsedMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

#[derive(Debug)]
pub struct MoveParser {}

impl MoveParser {
    pub fn new() -> Self {
        MoveParser {
        }
    }

    /**
      Handles the following cases:
      e2e4
      e4
      exd6 (e.p)
      Nf3
      Nxf3
      Bxc3
      Ng1xf3
      Rd8f8 or Rdf8
      Ra1a3 or R1a3
      e8=q
      O-O-O
      O-O
    */
    pub fn parse(&mut self, mv: &str) -> Option<ParsedMove> {
        if mv.len() != 4 { return None; }
        let from_coord_str2 = &mv[0..2];
        let to_coord_str2 = &mv[2..4];

        fn parse_sq(sq: &str) -> Option<(usize, usize)> {
            let bytes = sq.as_bytes();
            if bytes.len() != 2 { return None; }
            let file = (bytes[0] as char).to_ascii_lowercase();
            let rank = bytes[1] as char;

            if !('a'..='h').contains(&file) { return None; }
            if !('1'..='8').contains(&rank) { return None; }

            let col = (file as u8 - b'a') as usize; // a->0, h->7
            let row = (rank as u8 - b'1') as usize; // 1->0, 8->7
            Some((row, col))
        }

        let from = match parse_sq(from_coord_str2) { Some(v) => v, None => return None };
        let to   = match parse_sq(to_coord_str2) { Some(v) => v, None => return None };

        Some(ParsedMove {
            from,
            to,
        })
    }
}