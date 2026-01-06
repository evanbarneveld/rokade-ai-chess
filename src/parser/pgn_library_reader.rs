use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::parser::pgn_document::PGNDocument;

/// Streams PGN games one-by-one from a large PGN file without loading it entirely.
///
/// Usage:
/// - Create with `PGNLibraryReader::new_default()` to read from `pgn_database/LumbrasGigaBase_Online_2025.pgn_database`,
///   or `PGNLibraryReader::new(path)` for a custom file.
/// - Call `next_pgn()` repeatedly to retrieve `PGNDocument`s until it returns `Ok(None)`.
/// - Call `reset()` to start over from the beginning of the file.
pub struct PGNLibraryReader {
    path: PathBuf,
    reader: BufReader<File>,
    // internal buffer reused between calls
    game_buf: String,
}

impl PGNLibraryReader {
    /// Construct a reader for the given file path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)
            .map_err(|e| format!("Failed to open PGN library '{}': {}", path_buf.display(), e))?;
        let reader = BufReader::new(file);
        Ok(Self { path: path_buf, reader, game_buf: String::new() })
    }

    /// Construct a reader pointing to the repository PGN:
    /// `pgn_database/LumbrasGigaBase_Online_2025.pgn_database` relative to current working directory.
    pub fn new_default() -> Result<Self, String> {
        let path = PathBuf::from("../../pgn_database").join("LumbrasGigaBase_Online_2025.pgn_database");
        Self::new(path)
    }

    /// Reset the reader to the beginning of the file.
    pub fn reset(&mut self) -> Result<(), String> {
        let file = File::open(&self.path)
            .map_err(|e| format!("Failed to reopen PGN library '{}': {}", self.path.display(), e))?;
        self.reader = BufReader::new(file);
        self.game_buf.clear();
        Ok(())
    }

    /// Read the next complete PGN game from the stream, if any.
    /// Returns `Ok(None)` on EOF.
    pub fn next_pgn(&mut self) -> Result<Option<PGNDocument>, String> {
        self.game_buf.clear();

        // We accumulate lines until we encounter a game result token in the movetext
        // (1-0, 0-1, 1/2-1/2, *). Headers and comments are kept as-is; PGNDocument will strip them.
        let mut saw_any_content = false;
        let mut line = String::new();

        loop {
            line.clear();
            let n = self.reader.read_line(&mut line)
                .map_err(|e| format!("Failed to read from PGN library '{}': {}", self.path.display(), e))?;
            if n == 0 {
                // EOF
                return if saw_any_content {
                    // Return the last accumulated (possibly incomplete if no explicit result),
                    // PGNDocument::from_str will tokenize up to available movetext.
                    let doc = PGNDocument::from_str(&self.game_buf);
                    if doc.is_empty() {
                        return Ok(None);
                    }
                    Ok(Some(doc))
                } else {
                    Ok(None)
                }
            }

            // Skip leading BOM if present at first line of file/chunk
            if !saw_any_content && self.game_buf.is_empty() && line.starts_with('\u{feff}') {
                line = line.trim_start_matches('\u{feff}').to_string();
            }

            // Track whether we saw any non-empty content to know if we started a game
            if !line.trim().is_empty() {
                saw_any_content = true;
            }

            self.game_buf.push_str(&line);

            // Heuristic: once a result token appears in the accumulated text, we consider the game complete
            if line_contains_result_token(line.trim()) {
                let doc = PGNDocument::from_str(&self.game_buf);
                return Ok(Some(doc));
            }
        }
    }
}

fn line_contains_result_token(s: &str) -> bool {
    // Quick checks; require token boundaries (whitespace or line edges)
    // to reduce false positives.
    contains_token(s, "1-0") || contains_token(s, "0-1") || contains_token(s, "1/2-1/2") || contains_token(s, "*")
}

fn contains_token(hay: &str, tok: &str) -> bool {
    if tok == "*" {
        // Star can be part of comments rarely; still treat as token if appears standalone
        for part in hay.split_whitespace() { if part == "*" { return true; } }
        return false;
    }
    for part in hay.split_whitespace() {
        if part == tok { return true; }
    }
    false
}
