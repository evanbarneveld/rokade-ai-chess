mod game_state;
mod pieces;
mod board;
mod castling_rights;
mod game;

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

    println!("Welcome to chess. Type 'fen' to show FEN, 'quit' to quit.\n");

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
        if input.is_empty() { continue; }

        if game.move_piece_str(input) {
            println!("OK\n");
        } else {
            println!("Illegal or invalid move: '{}'. Try again.\n", input);
        }
    }
}







