use std::io;
use std::io::Write;

#[test]
fn test_mates_generic() {
    use chess::board::san_move::convert_move_to_san;
    use chess::generator::move_generator::generate_move_as_san;
    use chess::history::history::History;
    use chess::parser::parser::MoveParser;
    use chess::piece::piece_mover::PieceMover;
    use chess::search::playing_strength::PLAYING_STRENGTH_MAX;
    use chess::search::{set_deterministic, SearchMode};
    use chess::state::fen::reader::reset_from_fen;

    // Read bundled puzzle files (same directory as this test module)
    const INPUT_MI2: &str = include_str!("mate-in-2.txt");
    const INPUT_MI3: &str = include_str!("mate-in-3.txt");
    const INPUT_MI4: &str = include_str!("mate-in-4.txt");

    // Ensure determinism for reproducible results
    set_deterministic(true);

    // Helper to strip trailing annotations like +, #, !?, etc.
    fn clean_token(tok: &str) -> String {
        tok.trim()
            .trim_end_matches(['+', '#', '!', '?'])
            .to_string()
    }

    // Extract SAN tokens from a solution line like:
    // "1. Nf6+ gxf6 2. Bxf7#" -> ["Nf6+", "gxf6", "Bxf7#"]
    fn parse_solution_line(line: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for t in line.split_whitespace() {
            // Skip move numbers like "1." or "2."
            if (t.ends_with('.') && t[..t.len() - 1].chars().all(|c| c.is_ascii_digit()))
                // Also skip black-to-move indicators like "1..."
                || (t.ends_with("...") && t[..t.len() - 3].chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }
            out.push(t.to_string());
        }
        out
    }

    fn run_suite(input: &str, label: &str) -> (usize, usize) {
        let mut total: usize = 0;
        let mut solved: usize = 0;

        let mut lines = input.lines().enumerate().peekable();
        // Track the most recent header lines before each FEN:
        // - last_header_title: e.g., "Black Mates in 2." or "White Mates in 3."
        // - last_header_players: e.g., "A vs B, Event, Date"
        let mut last_header_title: Option<String> = None;
        let mut last_header_players: Option<String> = None;
        while let Some((i, line)) = lines.next() {
            let fen_line_candidate = line.trim();
            if fen_line_candidate.is_empty() {
                continue;
            }

            // Heuristic: a FEN line has multiple space-separated fields and contains '/'
            let looks_like_fen = fen_line_candidate.contains('/')
                && fen_line_candidate.split_whitespace().count() >= 6;
            if !looks_like_fen {
                // Classify header lines prior to FEN
                let trimmed = fen_line_candidate;
                let looks_like_solution = trimmed.starts_with("1.")
                    || trimmed.starts_with("1 ")
                    || trimmed.starts_with("1...");
                if !looks_like_solution {
                    // The title line typically starts with "Black Mates in" or "White Mates in"
                    let lower = trimmed.to_ascii_lowercase();
                    if lower.starts_with("black mates in") || lower.starts_with("white mates in") {
                        last_header_title = Some(trimmed.to_string());
                    } else {
                        last_header_players = Some(trimmed.to_string());
                    }
                }
                continue;
            }

            // We have a FEN; the next non-empty line should be the solution line
            let fen = fen_line_candidate;
            // If we have remembered headers for this puzzle, print them
            if let Some(t) = last_header_title.take() {
                println!("Puzzle: {}", t);
                io::stdout().flush().unwrap();
            }
            if let Some(p) = last_header_players.take() {
                println!("Players: {}", p);
                io::stdout().flush().unwrap();
            }
            let mut solution_line: Option<String> = None;
            // Look ahead up to a few lines to find the solution line (skip blanks)
            for _ in 0..5 {
                if let Some((_j, next_line)) = lines.peek().cloned() {
                    if next_line.trim().is_empty() {
                        lines.next();
                        continue;
                    }
                    // If it starts with a move number like "1." or "1..." consider it a solution
                    let trimmed = next_line.trim_start();
                    if trimmed.starts_with("1.")
                        || trimmed.starts_with("1 ")
                        || trimmed.starts_with("1...")
                    {
                        solution_line = Some(next_line.trim().to_string());
                        // consume it
                        lines.next();
                    }
                    break;
                }
            }

            if solution_line.is_none() {
                println!(
                    "Warning: Missing solution line near input line {} for FEN: {}",
                    i + 1,
                    fen
                );
                continue;
            }

            total += 1;

            let solution = solution_line.unwrap();
            let tokens: Vec<String> = parse_solution_line(&solution)
                .into_iter()
                .map(|t| clean_token(&t))
                .collect();
            if tokens.is_empty() {
                println!("Warning: No moves parsed for FEN: {}", fen);
                continue;
            }

            // Build initial state from FEN
            let mut gs = match reset_from_fen(fen) {
                Ok(g) => g,
                Err(_) => {
                    println!("Invalid FEN (skipped): {}", fen);
                    continue;
                }
            };
            let history = History::new();
            let mut parser = MoveParser::new();

            // Walk the solution line: engine must match all even-indexed moves; odd-indexed moves are opponent replies from the file.
            let mut matched_all_engine_moves = true;
            let mut move_index: usize = 0;
            while move_index < tokens.len() {
                // Engine move expected
                let expected_engine_san_str = &tokens[move_index];

                // Generate the best move; use a depth based on remaining plies (cap at 16) and some time buffer for the first move
                let depth = 20;
                let time_ms = 1000;
                let engine_move_san = generate_move_as_san(
                    SearchMode::Advanced,
                    gs,
                    &history,
                    depth as usize,
                    time_ms,
                    PLAYING_STRENGTH_MAX,
                );

                io::stdout().flush().unwrap();

                // Normalize expected SAN in the current position
                let expected_engine_move_san = {
                    let mut board = gs.board().clone();
                    let active = gs.active_color();
                    let enp = gs.en_passant_target();
                    match parser.parse(&mut board, active, expected_engine_san_str, enp) {
                        Ok(pm) => convert_move_to_san(gs, Some((pm.from, pm.to))),
                        Err(err) => {
                            println!(
                                "Parse error for expected move '{}' on FEN {}: {}",
                                expected_engine_san_str, fen, err
                            );
                            None
                        }
                    }
                };

                if !(engine_move_san.is_some()
                    && expected_engine_move_san.is_some()
                    && engine_move_san == expected_engine_move_san)
                {
                    println!("Engine move {} != expected move {}", engine_move_san.clone().unwrap(), expected_engine_san_str);

                    let engine_mov = engine_move_san.clone().unwrap();

                    // Qc1c3, expected move Qc3
                    if expected_engine_san_str.len() == 3 && engine_mov.len() == 5 && expected_engine_san_str.starts_with(&engine_mov[0..1]) && expected_engine_san_str.ends_with(&engine_mov[3..5]) {
                        println!("Move could be matched.");
                    } else {
                        matched_all_engine_moves = false;
                        break;
                    }
                } else {
                    println!("Engine move {} == expected move {}", engine_move_san.clone().unwrap(), expected_engine_san_str);

                }

                // Apply the engine move (according to the expected token) to advance the position
                {
                    let mut board = gs.board().clone();
                    let active = gs.active_color();
                    let enp = gs.en_passant_target();
                    if let Ok(pm) = parser.parse(&mut board, active, expected_engine_san_str, enp) {
                        let is_cap = board.get(pm.to.0, pm.to.1).is_some();
                        let _ = PieceMover::move_piece(
                            &mut gs,
                            pm.from,
                            pm.to,
                            is_cap,
                            pm.promotion_piece,
                        );
                        gs.switch_player_turn();
                    } else {
                        println!(
                            "Could not apply engine move '{}' for FEN {}",
                            expected_engine_san_str, fen
                        );
                        matched_all_engine_moves = false;
                        break;
                    }
                }

                move_index += 1;
                if move_index >= tokens.len() {
                    break;
                }

                // Opponent reply from line (no engine check here, just apply)
                let reply_san_str = &tokens[move_index];
                {
                    let mut board = gs.board().clone();
                    let active = gs.active_color();
                    let enp = gs.en_passant_target();
                    if let Ok(pm) = parser.parse(&mut board, active, reply_san_str, enp) {
                        let is_cap = board.get(pm.to.0, pm.to.1).is_some();
                        let _ = PieceMover::move_piece(
                            &mut gs,
                            pm.from,
                            pm.to,
                            is_cap,
                            pm.promotion_piece,
                        );
                        gs.switch_player_turn();
                    } else {
                        println!(
                            "Could not apply opponent reply '{}' for FEN {}",
                            reply_san_str, fen
                        );
                        matched_all_engine_moves = false;
                        break;
                    }
                }

                move_index += 1;
            }

            if matched_all_engine_moves {
                solved += 1;
            }

            println!("Current puzzle {}, solved {}", total, solved)
        }

        if total == 0 {
            println!("No puzzles found in {}", label);
        }

        let pct = if total > 0 {
            (solved as f64) * 100.0 / (total as f64)
        } else {
            0.0
        };
        println!("{}: solved {}/{} ({:.1}%)", label, solved, total, pct);
        (total, solved)
    }


    let (t2, s2) = run_suite(INPUT_MI2, "Mate-in-2");
    let (t3, s3) = run_suite(INPUT_MI3, "Mate-in-3");
    let (t4, s4) = run_suite(INPUT_MI4, "Mate-in-4");

    let total = t2 + t3 + t4;
    let solved = s2 + s3 + s4;

    assert!(total > 0, "No puzzles found in provided files");
    assert!(solved > 0, "Engine failed to solve any provided puzzles");
}
