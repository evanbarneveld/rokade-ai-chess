use std::io::{self, Write};
use chess::Chess;              // uses the re‑export from lib.rs
use chess::parser::pgn_document::PGNDocument;

fn main() {
    // Start a game from the initial position
    let mut game = Chess::new();

    if let Err(e) = game.reset() {
        eprintln!("Error parsing FEN: {}", e);
        return;
    }

    println!("Welcome to chess. Type 'fen' to show FEN, 'reset [<fen>] or reset standard' to reset the board, 'exit' to exit.\n");

    loop {
        println!("{}", game.board());

        if game.active_color_is_white() { print!("White> "); } else { print!("Black> "); }

        if io::stdout().flush().is_err() { break; }

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            println!("Bye!");
            break;
        }
        if input.eq_ignore_ascii_case("fen") {
            println!("{}\n", game.to_fen());
            continue;
        }
        // Handle "reset <fen>" to reinitialize the board from a FEN string
        {
            let mut parts = input.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            if cmd.eq_ignore_ascii_case("reset") {
                let fen = parts.next().unwrap_or("").trim();
                if fen.is_empty() {
                    match game.reset() {
                        Ok(_) => println!("Board reset.\n"),
                        Err(e) => println!("Error resetting board: {}\n", e),
                    }
                } else if fen.eq("standard") {
                    game = Chess::new();
                } else {
                    match game.set_starting_fen(fen) {
                        Ok(()) => println!("Board was reset.\n"),
                        Err(e) => println!("Error parsing FEN: {}\n", e),
                    }
                }
                continue;
            }
        }
        if input.is_empty() { continue; }

        // Handle "pgn <file>" to feed moves from a PGN file
        {
            let mut parts = input.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            if cmd.eq_ignore_ascii_case("pgn") {
                let path = parts.next().unwrap_or("").trim();
                if path.is_empty() {
                    println!("Usage: PGN <file>\n");
                    continue;
                }
                match PGNDocument::from_file(path) {
                    Ok(mut doc) => {
                        let mut applied = 0usize;
                        while let Some(mv) = doc.next_move() {
                            if !game.move_piece_san(&mv) {
                                println!("Illegal or invalid move from PGN: '{}'. Stopping.\n", mv);
                                break;
                            }
                            applied += 1;
                        }
                        println!("Applied {} move(s) from '{}'.\n", applied, path);
                    }
                    Err(e) => println!("{}\n", e),
                }
                continue;
            }
        }

        if !game.move_piece_san(input) {
            println!("Illegal or invalid move: '{}'. Try again.\n", input);
        }
    }
}







