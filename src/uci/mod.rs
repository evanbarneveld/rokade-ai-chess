use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::exit;
use std::thread;
use chrono::Local;
use crate::book::book::set_order_book_enabled;
use crate::Chess;
use crate::cli::{BUILD_NUMBER, VERSION};
use crate::piece::as_move_str;
use crate::search::core::advanced_search::{DEFAULT_SEARCH_DEPTH, MAX_SEARCH_DEPTH};
use crate::search::{
    find_best_move_with_mode, get_deterministic, is_parallel_search, set_deterministic,
    set_parallel_search, SearchMode,
};
use crate::search::core::advanced_search::get_order_book_enabled;
use crate::search::integration::telemetry::{get_nodes, reset_search_telemetry};
use crate::search::integration::time_control::{clear_time_budget, set_time_budget_ms};
use crate::search::integration::uci_feedback::set_info_callback;

// Minimal UCI interface implementation.
// Supported commands:
// - uci
// - isready
// - ucinewgame
// - position [startpos|fen <fen>] [moves <move1> <move2> ...]
// - go depth N
// - go wtime <time-in-ms> btime <time-in-ms> [ winc <time-in-ms> binc <time-in-ms>
// - stop (stop current search)
// - cli (switch back to CLI mode)
// - quit

const UCI_INFO_SCORE_DIVISOR: f32 = 8.0f32;

static LOG: OnceLock<Mutex<File>> = OnceLock::new();

pub fn run_uci() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let log_filename = get_log_filename();
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_filename)?;

    let _ = LOG.set(Mutex::new(log_file));

    let engine = Arc::new(Mutex::new(Chess::new()));
    let searching = Arc::new(AtomicBool::new(false));

    send_uci_response();

    // ensure starting position
    let _ = engine.lock().unwrap().reset();

    let mut input = String::new();

    loop {
        input.clear();
        let bytes = stdin.lock().read_line(&mut input)?;
        if bytes == 0 {
            break;
        }
        let line = input.trim();
        // log input
        log_with_flush("IN ", line);
        if line.is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("uci") {
            send_uci_response();
        }

        if line.to_ascii_lowercase().starts_with("setoption ") {
            // Handle: setoption name <SearchMode|Strength> value <...>
            let lower = line.to_ascii_lowercase();
            if lower.contains("name searchmode") {
                if let Some(idx) = lower.find("value ") {
                    let val = line[(idx + 6)..].trim();
                    let mode = match val.to_ascii_lowercase().as_str() {
                        "normal" => Some(SearchMode::Normal),
                        "test (slow)" => Some(SearchMode::Test),
                        _ => None,
                    };
                    if let Some(m) = mode { engine.lock().unwrap().set_search_mode(m); }
                }
            } else if lower.contains("name strength") {
                if let Some(idx) = lower.find("value ") {
                    // value is one of: Strength Max, Strength 9..1
                    let val = line[(idx + 6)..].trim();
                    let val_lower = val.to_ascii_lowercase();
                    let s = if val_lower == "strength max" {
                        1000usize
                    } else if val_lower == "strength 9" {
                        950usize
                    } else if val_lower == "strength 8" {
                        850usize
                    } else if val_lower == "strength 7" {
                        750usize
                    } else if val_lower == "strength 6" {
                        650usize
                    } else if val_lower == "strength 5" {
                        550usize
                    } else if val_lower == "strength 4" {
                        450usize
                    } else if val_lower == "strength 3" {
                        350usize
                    } else if val_lower == "strength 2" {
                        250usize
                    } else if val_lower == "strength 1" {
                        150usize
                    } else {
                        // Unknown value, ignore
                        engine.lock().unwrap().get_playing_strength()
                    };
                    engine.lock().unwrap().set_playing_strength(s);
                }
            } else if lower.contains("name deterministic") {
                if let Some(idx) = lower.find("value ") {
                    let val = line[(idx + 6)..].trim().to_ascii_lowercase();
                    if val == "true" {
                        set_deterministic(true);
                    } else if val == "false" {
                        set_deterministic(false);
                    }
                }
            } else if lower.contains("name parallel search") {
                if let Some(idx) = lower.find("value ") {
                    let val = line[(idx + 6)..].trim().to_ascii_lowercase();
                    if val == "true" {
                        set_parallel_search(true);
                    } else if val == "false" {
                        set_parallel_search(false);
                    }
                }
            } else if lower.contains("name order book")
                && let Some(idx) = lower.find("value ") {
                    let val = line[(idx + 6)..].trim().to_ascii_lowercase();
                    if val == "true" {
                        set_order_book_enabled(true);
                    } else if val == "false" {
                        set_order_book_enabled(false);
                    }
                }
            continue;
        }
        if line == "isready" {
            let m = "readyok".to_string();
            write_to_stdout_and_log_with_flush("OUT", &m);
            stdout.flush()?;
            continue;
        }
        if line == "ucinewgame" {
            let _ = engine.lock().unwrap().reset();
            continue;
        }
        if line.eq_ignore_ascii_case("board") {
            //display board
            let mut eng = engine.lock().unwrap();
            let history = eng.get_history().clone();
            println!("{}", eng.board().get_board_display_string(Some(&history)));
            continue;

        }
        if line.eq_ignore_ascii_case("fen") {
            let _ = writeln!(stdout, "{}", engine.lock().unwrap().to_fen());
            continue;
        }
        if line.starts_with("position ") {
            handle_position(&mut engine.lock().unwrap(), line.strip_prefix("position ").unwrap());
            continue;
        }
        if line.starts_with("go ") || line == "go" {
            if searching.load(Ordering::SeqCst) {
                write_to_stdout_and_log_with_flush("OUT", "info string already searching\n");
                continue;
            }
            searching.store(true, Ordering::SeqCst);

            let white_is_active = engine.lock().unwrap().active_color_is_white();

            let mut movetime: usize = 0; // default unlimited time per move unless constrained below

            // Parse tokens for movetime and/or wtime/btime and increments winc/binc (order-independent)
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let mut wtime: Option<usize> = None;
            let mut btime: Option<usize> = None;
            let mut winc: Option<usize> = None;
            let mut binc: Option<usize> = None;
            let mut movestogo: Option<usize> = None;

            let mut i = 1; // start after 'go'
            while i < tokens.len() {
                match tokens[i] {
                    "movetime" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() {
                                movetime = v;
                            }
                            i += 2; continue;
                        }
                    }
                    "wtime" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() { wtime = Some(v); }
                            i += 2; continue;
                        }
                    }
                    "btime" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() { btime = Some(v); }
                            i += 2; continue;
                        }
                    }
                    "winc" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() { winc = Some(v); }
                            i += 2; continue;
                        }
                    }
                    "binc" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() { binc = Some(v); }
                            i += 2; continue;
                        }
                    }
                    "movestogo" => {
                        if i + 1 < tokens.len() {
                            if let Ok(v) = tokens[i + 1].parse::<usize>() { movestogo = Some(v.max(1)); }
                            i += 2; continue;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            // If no explicit movetime was given but wtime/btime or winc/binc was provided, derive a reasonable per-move budget
            if movetime == 0 && (wtime.is_some() || btime.is_some() || winc.is_some() || binc.is_some()) {
                let time_left = if white_is_active { wtime.unwrap_or(0) } else { btime.unwrap_or(0) };
                let inc = if white_is_active { winc.unwrap_or(0) } else { binc.unwrap_or(0) };

                let mut budget: usize = 0;

                if time_left > 0 {
                    if let Some(mtg) = movestogo {
                        // If movestogo is provided, allocate roughly evenly across remaining moves.
                        // Base budget: divide remaining time by moves to go.
                        budget = (time_left / mtg.max(1)).max(1);
                        // Add a safe portion of the increment if available.
                        if inc > 0 {
                            let bonus = ((inc as f64) * 0.6) as usize; // ~60% of increment
                            budget = budget.saturating_add(bonus);
                        }
                        // Cap so we don't spend too much: at most time_left / max(2, mtg)
                        let max_cap = time_left / mtg.max(2);
                        if max_cap > 0 && budget > max_cap { budget = max_cap; }
                    } else {
                        // No movestogo: use a dynamic fraction of remaining time and some increment.
                        budget = ((time_left as f64) * 0.02) as usize; // ~2%
                        if budget < 10 { budget = 10; } // at least 10ms
                        if inc > 0 {
                            let bonus = ((inc as f64) * 0.7) as usize; // ~70% of increment
                            budget = budget.saturating_add(bonus);
                        }
                        let max_cap = time_left / 3; // at most a third of remaining time
                        if max_cap > 0 && budget > max_cap { budget = max_cap; }
                    }

                    // If in severe time trouble, be extra conservative regardless of movestogo
                    if time_left < 1000 { // <1s left
                        let tight = (time_left / 20).max(5); // ~5% of remaining, min 5ms
                        budget = budget.min(tight);
                    }
                } else if inc > 0 { // no main time, only increment
                    // When only increment is available, use most of it but not all
                    budget = ((inc as f64) * 0.9) as usize; // 90% of increment
                    if budget < 10 { budget = 10; }
                    // Cap at twice the increment to avoid overthinking
                    let max_cap = inc.saturating_mul(2);
                    if budget > max_cap { budget = max_cap; }
                }

                if budget > 0 { movetime = budget; }
            }

            let engine_inner = Arc::clone(&engine);
            let searching_inner = Arc::clone(&searching);
            let line_copy = line.to_string();
            thread::spawn(move || {
                // Measure elapsed time and reset telemetry for info lines
                let start = std::time::Instant::now();
                reset_search_telemetry();
                // Install a temporary info callback so we can emit progress while searching.
                // The callback prints UCI-compliant info lines with the current best root move scores.
                let white_is_active_inner = white_is_active;
                let info_cb = Arc::new(move |_mv: ((usize, usize), (usize, usize), Option<char>), score_cp: i32, depth_used: usize, pv_moves: Vec<((usize, usize), (usize, usize), Option<char>)>, hashfull: u16| {
                    let mut pv_parts: Vec<String> = Vec::with_capacity(pv_moves.len());
                    for (f, t, p) in pv_moves {
                        let mut m_str = as_move_str(f, t);
                        if let Some(c) = p {
                            m_str.push(c);
                        }
                        pv_parts.push(m_str);
                    }
                    let pv = pv_parts.join(" ");
                    // Compute current nodes and nps
                    let nodes = get_nodes();
                    let ms = start.elapsed().as_millis().max(1);
                    let nps = (nodes as u128 * 1000u128) / ms;
                    // Print directly to stdout; ignore logging for async updates
                    let score_cp_from_white_perspective = if white_is_active_inner { score_cp } else { -score_cp };
                    let log_text = format!("info depth {} score cp {} nodes {} nps {} hashfull {} pv {}", depth_used, (score_cp_from_white_perspective as f32 / UCI_INFO_SCORE_DIVISOR) as i32, nodes, nps, hashfull, pv);
                    write_to_stdout_and_log_with_flush("OUT", &log_text);
                });
                set_info_callback(Some(info_cb));

                // Apply a time budget for this Search
                let (best_move_str, info_opt) = {
                    let mut engine_locked = engine_inner.lock().unwrap();
                    go_bestmove_with_info(&mut engine_locked, &line_copy, movetime)
                };
                // Clear the callback after Search completes
                set_info_callback(None);
                let elapsed_ms = start.elapsed().as_millis();
                let nodes = get_nodes();
                let nps = if elapsed_ms == 0 { 0 } else { ((nodes as u128 * 1000u128) / elapsed_ms) as u128 };

                // If we have extra info from the Search, emit a UCI info line
                if let Some((score_cp, depth_used)) = info_opt {
                    let score_cp_from_white_perspective = if white_is_active { score_cp } else { -score_cp };
                    let log_text = format!("info depth {} score cp {} time {} nodes {} nps {} pv {}", depth_used, (score_cp_from_white_perspective as f32 / UCI_INFO_SCORE_DIVISOR) as i32, elapsed_ms, nodes, nps, best_move_str);
                    write_to_stdout_and_log_with_flush("OUT", &log_text);
                }

                let out = format!("bestmove {}", best_move_str);
                write_to_stdout_and_log_with_flush("OUT", &out);
                searching_inner.store(false, Ordering::SeqCst);
            });
            continue;
        }
        if line == "stop" {
            set_time_budget_ms(1);
            continue;
        }
        if line == "quit" {
            exit(0);
        }

        if line == "cli" {
            break;
        }
    }

    Ok(())
}

fn send_uci_response() {
    let m1 = format!("id name Rokade-AI v{} (build#{})", VERSION, BUILD_NUMBER).to_string();
    write_to_stdout_and_log_with_flush("OUT", &m1);

    let m2 = "id author Erik van Barneveld".to_string();
    write_to_stdout_and_log_with_flush("OUT", &m2);

    // Strength levels as combo
    let opt_strenghth = "option name Strength type combo default Strength Max var Strength Max var Strength 9 var Strength 8 var Strength 7 var Strength 6 var Strength 5 var Strength 4 var Strength 3 var Strength 2 var Strength 1".to_string();
    write_to_stdout_and_log_with_flush("OUT", &opt_strenghth);

    let opt_parallel = format!("option name parallel search type check default {}", is_parallel_search());
    write_to_stdout_and_log_with_flush("OUT", &opt_parallel);

    let opt_deterministic = format!("option name Deterministic type check default {}", get_deterministic());
    write_to_stdout_and_log_with_flush("OUT", &opt_deterministic);

    let opt_order_book = format!("option name Order Book type check default {}", get_order_book_enabled());
    write_to_stdout_and_log_with_flush("OUT", &opt_order_book);

    let opt_searchmode = "option name SearchMode type combo default Normal var Normal var Test (slow)".to_string();
    write_to_stdout_and_log_with_flush("OUT", &opt_searchmode);

    let m3 = "uciok".to_string();
    write_to_stdout_and_log_with_flush("OUT", &m3)
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
    let promo = if mv.len() >= 5 { mv.chars().nth(4) } else { None };

    let parse = |sq: &str| -> Option<(usize, usize)> {
        let bytes = sq.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = (bytes[0] as char).to_ascii_lowercase();
        let rank = bytes[1] as char;
        if !('a'..='h').contains(&file) {
            return None;
        }
        if !('1'..='8').contains(&rank) {
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

    engine.move_piece(from_idx, to_idx, promo)
}

pub fn go_bestmove_with_info(engine: &mut Chess, line: &str, move_time_in_ms: usize) -> (String, Option<(i32, usize)>) {
    // Similar to go_bestmove but also returns (score_cp, depth_used) for UCI info line.
    let mut depth = parse_depth(line).unwrap_or(DEFAULT_SEARCH_DEPTH);
    if depth > MAX_SEARCH_DEPTH { depth = MAX_SEARCH_DEPTH; }

    let gs_copy = { *engine.get_game_state() };
    let history_clone = { engine.get_history().clone() };
    // Use the configured playing strength; time control will be enforced by Search budget.
    let playing_strength = engine.get_playing_strength();

    /* TODO: find a better way to set the stength: use an UCI option!
    //if move_time is set a value below 1000mS. Then the user clearly wants to use low strength.
    //So in this case, pass use the move_time as 'strength' and override the move_time to some default
    //value that is reasonable for low strength.

    if move_time_in_ms < MAX_PLAYING_STRENGTH {
        playing_strength = move_time_in_ms;
        move_time_in_ms = DEFAULT_MOVE_TIME_FOR_STRENGTH_MODE_PLAY;
    }*/

    // Apply a time budget for this Search
    set_time_budget_ms(move_time_in_ms);
    let best = find_best_move_with_mode(engine.get_search_mode(), &gs_copy, &history_clone, depth, playing_strength);
    clear_time_budget();

    if let Some(((fr, fc), (tr, tc), promo, score_cp, depth_used)) = best {
        let mut mv = as_move_str((fr, fc), (tr, tc));
        if let Some(c) = promo {
            mv.push(c);
        }
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

fn get_log_filename() -> String {
    let base_name = "rokade-ai-chess";

    let mut i = 1;
    loop {
        let candidate = format!("{}.{}.log", base_name, i);
        if !Path::new(&candidate).exists() {
            return candidate;
        }
        i += 1;
    }
}

fn write_to_stdout_and_log_with_flush(direction: &str, text: &str) {
    writeln!(io::stdout(), "{}", text).expect("should have written to stdout");
    io::stdout().flush().expect("should have flushed stdout");
    log_with_flush(direction, text);
}

fn log_with_flush(direction: &str, text: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    if let Some(mutex) = LOG.get()
        && let Ok(mut log) = mutex.lock() {
            let _ = writeln!(log, "[{}] [{:5}] [{}] {}", now, std::process::id(), direction, text);
            let _ = log.flush();
        }
}
