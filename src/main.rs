mod game_state;
mod pieces;
mod board;
mod game;
mod piece_mover;

use crate::game::Game;
use std::io::{self, Write};

fn main() {
    // Start a game from the initial position
    let mut game = Game::new();

    let starting_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    if let Err(e) = game.init_state_from_fen(starting_fen) {
        eprintln!("Error parsing FEN: {}", e);
        return;
    }

    println!("Welcome to chess. Type 'fen' to show FEN, 'init <fen>' to init board, 'quit' to quit.\n");

    loop {
        println!("{}", game.board());
        print!("Move> ");
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
                    println!("Usage: init <FEN>\n");
                } else {
                    match game.init_state_from_fen(fen) {
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







