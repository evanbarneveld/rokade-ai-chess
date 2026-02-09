use chess::piece::pieces::Color;
use chess::state::outcome::OutcomeType;
use chess::Chess;
use std::fs::File;
use std::io;
use std::io::Write;

#[test]
fn test_mate_in_n_generic() {
    use chess::generator::move_generator::generate_move_as_san;
    use chess::search::{SearchContext, SearchMode};

    // Read bundled puzzle files (same directory as this test module)
    const INPUT_MI2: &str = include_str!("mate-in-2.txt");
    const INPUT_MI3: &str = include_str!("mate-in-3.txt");
    const INPUT_MI4: &str = include_str!("mate-in-4.txt");

    // Ensure determinism for reproducible results
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);

    // Helper to strip trailing annotations like +, #, !?, etc.
    fn clean_token(tok: &str) -> String {
        tok.trim().trim_end_matches(['!', '?']).to_string()
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

    fn run_suite(
        game: &mut Chess,
        ctx: &SearchContext,
        input: &str,
        label: &str,
        max_moves: usize,
        max_search_depth: usize,
        move_time_in_ms: usize,
        max_puzzles: i32,
    ) -> (usize, usize) {
        if max_puzzles < 0 {
            return (0, 0);
        }

        let mut total: usize = 0;
        let mut solved: usize = 0;
        let mut failure_count: usize = 0;

        // Create/truncate the failure file at the start
        let filename = format!("mate_in_{}_failures.txt", max_moves);

        let mut file = File::create(filename.clone()).ok().unwrap();

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
            let current_title = if let Some(t) = last_header_title.take() {
                println!("Puzzle: {}", t);
                io::stdout().flush().unwrap();
                t
            } else {
                format!("Puzzle at line {}", i + 1)
            };
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

            game.set_starting_fen(&fen)
                .expect("Could not set starting FEN");

            let mut ply_count = 1;

            loop {
                let gs = game.get_game_state().clone();
                let hs = game.get_history().clone();

                let engine_move_san = generate_move_as_san(
                    ctx,
                    SearchMode::Normal,
                    &gs,
                    &hs,
                    max_search_depth,
                    move_time_in_ms,
                    1000,
                );
                if !game.move_piece_san(&engine_move_san.clone().unwrap()) {
                    println!("Warning: Invalid move {}", engine_move_san.unwrap());
                } else {
                    println!("{} ", engine_move_san.unwrap());
                }

                ply_count += 1;

                game.get_game_state().recompute_outcome(&hs);
                let outcome = game.get_game_state().get_outcome().unwrap();

                if outcome.eq(&OutcomeType::Checkmate {
                    winner: Color::Black,
                }) || outcome.eq(&OutcomeType::Checkmate {
                    winner: Color::White,
                }) {
                    println!(
                        " ... mate in {} moves, Outcome: {:?}",
                        ply_count / 2,
                        outcome
                    );
                    solved += 1;
                    println!("Current puzzle {}, solved {}", total, solved);
                    break;
                }

                if ply_count >= max_moves * 2 {
                    println!("Failed finding find mate in {} moves.", max_moves);

                    failure_count += 1;
                    writeln!(file, "{}", current_title).ok();
                    if let Some(p) = last_header_players.take() {
                        writeln!(file, "{}", p).ok();
                    }
                    writeln!(file, "FEN: {}", fen).ok();
                    writeln!(file, "Solution: {}", solution).ok();
                    writeln!(file, "").ok();
                    file.flush().ok();

                    break;
                }
            } //loop

            if max_puzzles > 0 && total as i32 >= max_puzzles {
                break;
            }
        }

        // Write summary at the end
        if failure_count > 0 {
            writeln!(file, "---").ok();
            writeln!(file, "Summary: {}/{} puzzles failed", failure_count, total).ok();
            file.flush().ok();
            println!("Wrote {} failures to {}", failure_count, filename);
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

    let mut game = Chess::new();

    let mut total = 0;
    let mut solved = 0;

    //puzzles_to_solve = <0 means skip, 0 means all, >0 means number of puzzles to solve

    //------------------
    // MATE_IN_2 PUZZLES
    //------------------
    let puzzles_to_solve = -1;
    let move_time = 100;
    let (t, s) = run_suite(
        &mut game,
        &ctx,
        INPUT_MI2,
        "Mate-in-2",
        2,
        4,
        move_time,
        puzzles_to_solve,
    );
    if (s as f32 / t as f32) < 0.96 {
        panic!(
            "Failed to solve enough mate in 2 puzzles. Increase time or depth or improve engine"
        );
    }
    total += t;
    solved += s;

    //------------------
    // MATE_IN_3 PUZZLES
    //------------------
    let puzzles_to_solve = -1; //none
    let move_time = 2000;
    let (t, s) = run_suite(
        &mut game,
        &ctx,
        INPUT_MI3,
        "Mate-in-3",
        3,
        6,
        move_time,
        puzzles_to_solve,
    );
    if (s as f32 / t as f32) < 0.92 {
        panic!(
            "Failed to solve enough mate in 3 puzzles. Increase time or depth or improve engine"
        );
    }
    total += t;
    solved += s;

    //------------------
    // MATE_IN_4 PUZZLES
    //------------------
    let puzzles_to_solve = -1; //none
    let move_time = 3000;
    let (t, s) = run_suite(
        &mut game,
        &ctx,
        INPUT_MI4,
        "Mate-in-4",
        4,
        8,
        move_time,
        puzzles_to_solve,
    );
    if (s as f32 / t as f32) < 0.84 {
        panic!(
            "Failed to solve enough mate in 4 puzzles. Increase time or depth or improve engine"
        );
    }

    total += t;
    solved += s;

    assert_eq!(solved, total, "Engine failed to solve all provided puzzles");
}
