use std::io::{self, Write};
use chess::Chess;              // uses the re‑export from lib.rs

fn main() {
    // Start a game from the initial position
    let mut game = Chess::new();

    if let Err(e) = game.reset() {
        eprintln!("Error parsing FEN: {}", e);
        return;
    }

    println!("Welcome to chess. Type 'fen' to show FEN, 'init [<fen>]' to init board, 'quit' to quit.\n");

    loop {
        println!("{}", game.board());

        if game.active_color_is_white() { print!("White> "); } else { print!("Black> "); }

        if io::stdout().flush().is_err() { break; }

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") {
            println!("Bye!");
            break;
        }
        if input.eq_ignore_ascii_case("fen") {
            println!("{}\n", game.to_fen());
            continue;
        }
        // Handle "init <fen>" to reinitialize the board from a FEN string
        {
            let mut parts = input.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            if cmd.eq_ignore_ascii_case("init") {
                let fen = parts.next().unwrap_or("").trim();
                if fen.is_empty() {
                    match game.reset() {
                        Ok(GameState) => println!("Board reinitialized.\n"),
                        Err(e) => println!("Error reinitializing board: {}\n", e),
                    }
                } else {
                    match game.set_starting_fen(fen) {
                        Ok(()) => println!("Board reinitialized.\n"),
                        Err(e) => println!("Error parsing FEN: {}\n", e),
                    }
                }
                continue;
            }
        }
        if input.is_empty() { continue; }

        if game.move_piece_str(input) {
            println!("OK\n");
        } else {
            println!("Illegal or invalid move: '{}'. Try again.\n", input);
        }
    }
}







