use std::fs;
use std::path::Path;
use std::fmt;

/// A very lightweight PGN reader that extracts SAN-like move tokens in order.
///
/// Supported features:
/// - Skips headers like `[Event "..."]`
/// - Strips comments `{ ... }` (including time comments like `{[%clk 1:30:55]}`)
///   and `;` to end of line
/// - Removes NAGs like `$1`
/// - Ignores move numbers like `12.` and `12...`
/// - Stops at result tokens: `1-0`, `0-1`, `1/2-1/2`, `*`
/// - Returns one token per call to `next_move()` (e.g., `e4`, `Nf3`, `O-O`, `exd5`, `a8=Q`)
///
/// This reader does not validate moves against a board. It only tokenizes PGN text.
#[derive(Debug, Clone)]
pub struct PGNDocument {
    moves: Vec<String>,
    idx: usize,
}

impl PGNDocument {
    /// Load a PGN from disk and prepare the move stream.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read PGN file '{}': {}", path.as_ref().display(), e))?;
        Ok(Self::from_str(&text))
    }

    /// Create from raw PGN string content.
    pub fn from_str(pgn: &str) -> Self {
        // 1) Remove headers: lines starting with '[' and ending with ']'
        let mut body = String::with_capacity(pgn.len());
        for line in pgn.lines() {
            let trimmed = line.trim();
            if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
                body.push_str(line);
                body.push('\n');
            }
        }

        // 2) Remove comments in braces { ... }
        let mut no_brace_comments = String::with_capacity(body.len());
        let mut in_brace = false;
        for ch in body.chars() {
            match (in_brace, ch) {
                (false, '{') => in_brace = true,
                (true, '}') => in_brace = false,
                (false, c) => no_brace_comments.push(c),
                (true, _) => { /* skip */ }
            }
        }

        // 3) Remove ';' comments to end of line
        let mut no_line_comments = String::with_capacity(no_brace_comments.len());
        for line in no_brace_comments.lines() {
            if let Some(pos) = line.find(';') {
                no_line_comments.push_str(&line[..pos]);
            } else {
                no_line_comments.push_str(line);
            }
            no_line_comments.push('\n');
        }

        // 4) Tokenize by whitespace, then filter
        let mut moves: Vec<String> = Vec::new();
        for raw in no_line_comments.split_whitespace() {
            let tok = raw.trim();
            if tok.is_empty() { continue; }

            // Stop on result tokens
            if tok == "1-0" || tok == "0-1" || tok == "1/2-1/2" || tok == "*" {
                break;
            }

            // Skip NAGs
            if tok.starts_with('$') && tok[1..].chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            // Skip move numbers like "12." or "12..."
            if Self::looks_like_move_number(tok) { continue; }

            // Some PGN exports place black's ellipsis as a standalone token: "1. ... d5".
            // Ignore a bare ellipsis token so it doesn't get treated as a move.
            if tok == "..." { continue; }

            // Strip trailing check/mate/annotation symbols from token edge (we keep basic SAN core)
            //let clean = Self::strip_trailing_symbols(tok);
            let clean = tok.to_string();
            if clean.is_empty() { continue; }

            moves.push(clean.to_string());
        }

        PGNDocument { moves, idx: 0 }
    }

    /// Reset the iteration index back to the start of the move list.
    pub fn reset(&mut self) {
        self.idx = 0;
    }

    /// Get the next SAN token from the PGN or None when exhausted.
    pub fn next_move(&mut self) -> Option<String> {
        if self.idx >= self.moves.len() { return None; }
        let mv = self.moves[self.idx].clone();
        self.idx += 1;
        Some(mv)
    }

    /// Number of parsed SAN tokens.
    pub fn len(&self) -> usize { self.moves.len() }
    pub fn is_empty(&self) -> bool { self.moves.is_empty() }

    fn looks_like_move_number(tok: &str) -> bool {
        // Accept forms like: 1., 23... possibly with parentheses stripped earlier.
        let mut digits = 0usize;
        for c in tok.chars() {
            if c.is_ascii_digit() { digits += 1; continue; }
            if c == '.' { return digits > 0; }
            // otherwise not a move number
            return false;
        }
        false
    }
}

impl fmt::Display for PGNDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Emit a minimal, valid PGN movetext with result.
        // Example: "1. e4 e5 2. Nf3 Nc6 *"
        if self.moves.is_empty() {
            return write!(f, "*");
        }

        let mut first_pair = true;
        for (i, mv) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                // White move: start a new move number
                if !first_pair { write!(f, " ")?; }
                let num = i / 2 + 1;
                write!(f, "{}. {}", num, mv)?;
                first_pair = false;
            } else {
                // Black move
                write!(f, " {}", mv)?;
            }
        }

        write!(f, " *")
    }
}
