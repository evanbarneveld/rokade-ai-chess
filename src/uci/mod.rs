use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::fs::OpenOptions;
use crate::Chess;
use crate::piece::as_move_str;
use crate::search::search::{find_move_with_info, set_info_callback, get_nodes, reset_search_telemetry};

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
// - Promotions are assumed to be to a queen. Other promotion pieces are ignored for now.=

pub fn run_uci() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("uci.log")?;
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
        // log input
        log_io(&mut log, "IN ", line);
        if line.is_empty() {
            continue;
        }

        if line == "uci" {
            let m1 = "id name eriks-chess".to_string();
            writeln!(stdout, "{}", m1)?; log_io(&mut log, "OUT", &m1);
            let m2 = "id author erik van barneveld".to_string();
            writeln!(stdout, "{}", m2)?; log_io(&mut log, "OUT", &m2);
            let m3 = "uciok".to_string();
            writeln!(stdout, "{}", m3)?; log_io(&mut log, "OUT", &m3);
            stdout.flush()?;
            continue;
        }
        if line == "isready" {
            let m = "readyok".to_string();
            writeln!(stdout, "{}", m)?; log_io(&mut log, "OUT", &m);
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
            let mut movetime = 1000;

            let line_parts: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
            if line_parts.len() > 1 && line_parts[1] == "movetime" {
                if line_parts.len() > 2 {
                    //convert line_parts[2] to a number
                    movetime = line_parts[2].parse::<usize>().unwrap();
                }
            }

            // Measure elapsed time and reset telemetry for info lines
            let start = std::time::Instant::now();
            reset_search_telemetry();
            // Install a temporary info callback so we can emit progress while searching.
            // The callback prints UCI-compliant info lines with current best root move scores.
            let info_cb = Arc::new(move |_mv: ((usize, usize), (usize, usize)), score_cp: i32, depth_used: usize, pv_moves: Vec<((usize, usize), (usize, usize))>| {
                let mut pv_parts: Vec<String> = Vec::with_capacity(pv_moves.len());
                for (f, t) in pv_moves {
                    pv_parts.push(as_move_str(f, t));
                }
                let pv = pv_parts.join(" ");
                // Compute current nodes and nps
                let nodes = get_nodes();
                let ms = start.elapsed().as_millis().max(1);
                let nps = (nodes as u128 * 1000u128) / ms;
                // Print directly to stdout; ignore logging for async updates
                let _ = writeln!(io::stdout(), "info depth {} score cp {} nodes {} nps {} pv {}", depth_used, score_cp, nodes, nps, pv);
            });
            set_info_callback(Some(info_cb));

            let (best_move_str, info_opt) = go_bestmove_with_info(&mut engine, line, movetime);
            // Clear the callback after search completes
            set_info_callback(None);
            let elapsed_ms = start.elapsed().as_millis();
            let nodes = get_nodes();
            let nps = if elapsed_ms == 0 { 0 } else { ((nodes as u128 * 1000u128) / elapsed_ms) as u128 };

            // If we have extra info from the search, emit a UCI info line
            if let Some((score_cp, depth_used)) = info_opt {
                let info = format!("info depth {} score cp {} time {} nodes {} nps {} pv {}", depth_used, (score_cp as f32)/3.0f32, elapsed_ms, nodes, nps, best_move_str);
                writeln!(stdout, "{}", info)?; log_io(&mut log, "OUT", &info);
            }

            let out = format!("bestmove {}", best_move_str);
            writeln!(stdout, "{}", out)?; log_io(&mut log, "OUT", &out);
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
    let mut rest = args.trim().to_string();
    if let Some(s) = rest.strip_prefix("startpos") {
        let _ = engine.set_starting_fen(Chess::DEFAULT_CHESS_STARTING_FEN);
        let _ = engine.reset();
        rest = s.trim().to_string();
    } else if let Some(s) = rest.strip_prefix("fen ") {
        // everything up to " moves" (if present) is the FEN
        if let Some(idx) = s.find(" moves ") {
            let (fen, tail) = s.split_at(idx);
            let _ = engine.reset_board_to_fen(fen.trim());
            rest = format!("{} {}", "moves", tail.trim_start_matches(" moves "));
        } else {
            // only fen provided
            let _ = engine.reset_board_to_fen(s.trim());
            rest = String::new();
        }
    }

    if let Some(moves_part) = rest.as_str().strip_prefix("moves ") {
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

pub fn go_bestmove_with_info(engine: &mut Chess, line: &str, move_time: usize) -> (String, Option<(i32, usize)>) {
    // Similar to go_bestmove but also returns (score_cp, depth_used) for UCI info line.
    let mut depth = parse_depth(line).unwrap_or(4);
    if depth > 12 { depth = 12; }

    let gs_copy = { *engine.get_game_state() };
    let history_clone = { engine.get_history().clone() };
    let playing_strength = move_time;

    let best = find_move_with_info(gs_copy, &history_clone, depth, playing_strength);
    if let Some(((fr, fc), (tr, tc), score_cp, depth_used)) = best {
        let mv = as_move_str((fr, fc), (tr, tc));
        return (mv, Some((score_cp, depth_used)));
    }
    (String::from("0000"), None)
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

fn log_io(log: &mut std::fs::File, direction: &str, text: &str) {
    let _ = writeln!(log, "[{}] {}", direction, text);
    let _ = log.flush();
}