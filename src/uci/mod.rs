use std::io::{self, BufRead, Write};
use crate::Chess;
use crate::piece::as_move_str;
use crate::search::search::find_best_move;

// Minimal UCI interface implementation.
// Supported commands:
// - uci
// - isready
// - ucinewgame
// - position [startpos|fen <fen>] [moves <move1> <move2> ...]
// - go depth N
// - stop (no-op for instant search)
// - quit
// Notes:
// - Promotions are assumed to be to a queen. Other promotion pieces are ignored for now.
pub fn run_uci() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut engine = Chess::new();
    let mut running = true;

    // ensure starting position
    let _ = engine.reset();

    let mut input = String::new();
    loop {
        input.clear();
        let bytes = stdin.lock().read_line(&mut input)?;
        if bytes == 0 {
            break;
        }
        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if line == "uci" {
            writeln!(stdout, "id name eriks-chess")?;
            writeln!(stdout, "id author erik van barneveld")?;
            writeln!(stdout, "uciok")?;
            stdout.flush()?;
            continue;
        }
        if line == "isready" {
            writeln!(stdout, "readyok")?;
            stdout.flush()?;
            continue;
        }
        if line == "ucinewgame" {
            let _ = engine.reset();
            continue;
        }
        if line.starts_with("position ") {
            handle_position(&mut engine, line.strip_prefix("position ").unwrap());
            continue;
        }
        if line.starts_with("go ") || line == "go" {
            let best = go_bestmove(&mut engine, line);
            writeln!(stdout, "bestmove {}", best)?;
            stdout.flush()?;
            continue;
        }
        if line == "stop" {
            // no search thread yet
            continue;
        }
        if line == "quit" {
            running = false;
        }

        if !running {
            break;
        }
    }

    Ok(())
}

fn handle_position(engine: &mut Chess, args: &str) {
    // Syntax we handle:
    // position startpos [moves ...]
    // position fen <fen-string> [moves ...]
    let mut rest = args.trim();
    if let Some(s) = rest.strip_prefix("startpos") {
        let _ = engine.set_starting_fen(Chess::DEFAULT_CHESS_STARTING_FEN);
        let _ = engine.reset();
        rest = s.trim();
    } else if let Some(s) = rest.strip_prefix("fen ") {
        // everything up to " moves" (if present) is the FEN
        if let Some(idx) = s.find(" moves ") {
            let (fen, tail) = s.split_at(idx);
            let _ = engine.reset_board_to_fen(fen.trim());
            rest = tail.trim_start_matches(" moves ");
        } else {
            // only fen provided
            let _ = engine.reset_board_to_fen(s.trim());
            rest = "";
        }
    }

    if let Some(moves_part) = rest.strip_prefix("moves ") {
        for mv in moves_part.split_whitespace() {
            let _ = apply_uci_move(engine, mv);
        }
    }
}

fn apply_uci_move(engine: &mut Chess, mv: &str) -> bool {
    // UCI coordinate move: e2e4, e7e8q, g1f3, etc.
    if mv.len() < 4 {
        return false;
    }
    let from = &mv[0..2];
    let to = &mv[2..4];
    let _promo = if mv.len() >= 5 { Some(&mv[4..5]) } else { None };

    let parse = |sq: &str| -> Option<(usize, usize)> {
        let bytes = sq.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = (bytes[0] as char).to_ascii_lowercase();
        let rank = bytes[1] as char;
        if file < 'a' || file > 'h' {
            return None;
        }
        if rank < '1' || rank > '8' {
            return None;
        }
        let col = (file as u8 - b'a') as usize;
        let row = (rank as u8 - b'1') as usize; // board ranks 1..8 -> 0..7
        Some((row, col))
    };

    let from_idx = match parse(from) {
        Some(v) => v,
        None => return false,
    };
    let to_idx = match parse(to) {
        Some(v) => v,
        None => return false,
    };

    engine.move_piece(from_idx, to_idx)
}

fn go_bestmove(engine: &mut Chess, line: &str) -> String {
    // Currently we only support depth-limited instant move selection: pick the first legal move.
    // Parse optional depth but ignore for now.
    let depth = parse_depth(line).unwrap_or(4);
    let active = engine.get_game_state().active_color();
    let board = engine.board().clone();

    let best_move = find_best_move(&board, active, depth);

    if best_move.is_some() {
        let mv = best_move.unwrap();
        return as_move_str(mv.0, mv.1);
    }

    // No legal moves; signal no move.
    String::from("0000")
}

fn parse_depth(s: &str) -> Option<usize> {
    // e.g., "go depth 3"
    let mut it = s.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == "depth" {
            return it.next().and_then(|n| n.parse::<usize>().ok());
        }
    }
    None
}
