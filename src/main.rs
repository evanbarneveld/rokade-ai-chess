use std::io::{self, Write};
use chess::board::evaluator::evaluate_position;
use chess::Chess;              // uses the re‑export from lib.rs
use chess::pgn_player::pgn_player::PgnPlayer;
use chess::state::outcome::OutcomeType;
use chess::generator::move_generator::generate_move_as_san;

fn main() {
    // Start a game from the initial position
    let mut game = Chess::new();

    if let Err(e) = game.reset() {
        eprintln!("Error parsing FEN: {}", e);
        return;
    }

    println!("Welcome to chess. Type 'fen' to show FEN, 'reset [<fen>] or reset standard' to reset the board, 'exit' to exit.\n");

    println!("{}", game.board());

    let mut player_vs_player = true;
    let mut player_vs_bot = false;
    let mut bot_vs_player = false;
    let mut bot_vs_bot = false;

    loop {

        let game_outcome = game.get_game_state().get_outcome();
        let move_number = game.get_game_state().full_move_number();

        if game_outcome != Some(OutcomeType::Ongoing) && game_outcome != Some(OutcomeType::InCheck){
            print!("Game over: {:?}\nCommand> ", game_outcome.unwrap());
        } else {
            if game_outcome == Some(OutcomeType::InCheck) { print!("Check! "); }
            if game.active_color_is_white() {
                print!("White move {} > ", move_number);
            } else {
                print!("Black move {} > ", move_number);
            }
        }

        if io::stdout().flush().is_err() { break; }

        let mut input = String::from("");

        if (game.active_color_is_white() && (player_vs_player || player_vs_bot)) ||
            (!game.active_color_is_white() && (player_vs_player || bot_vs_player)) {
            let mut input_line = String::new();
            if io::stdin().read_line(&mut input_line).is_err() { break; }
            input = input_line.trim().to_string();
        }

        if input.eq_ignore_ascii_case("exit") {
            println!("Bye!");
            break;
        }
        if input.eq_ignore_ascii_case("pvsb") {
            player_vs_bot = true;
            player_vs_player = false;
            bot_vs_player = false;
            bot_vs_bot = false
        }
        if input.eq_ignore_ascii_case("bvsb") {
            bot_vs_bot = true;
            player_vs_player = false;
            bot_vs_player = false;
            player_vs_bot = false;
        }
        if input.eq_ignore_ascii_case("bvsp") {
            bot_vs_player = true;
            player_vs_player = false;
            player_vs_bot = false;
            bot_vs_bot = false
        }
        if input.eq_ignore_ascii_case("fen") {
            println!("{}\n", game.to_fen());
            println!("{}", game.board());
            continue;
        }
        if input.eq_ignore_ascii_case("undo") {
            game.undo_move();
            println!("{}", game.board());
            continue;
        }
        if input.eq_ignore_ascii_case("list") {
            game.list();
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
                        Ok(_) => {
                            println!("Board was reset.\n");
                            println!("{}", game.board());
                        },
                        Err(e) => println!("Error resetting board: {}\n", e),
                    }
                } else if fen.eq("standard") {
                    game = Chess::new();
                } else {
                    match game.set_starting_fen(fen) {
                        Ok(()) => {
                            println!("Board was reset.\n");
                            println!("{}", game.board())
                        },
                        Err(e) => println!("Error parsing FEN: {}\n", e),
                    }
                }
                continue;
            }
        }

        // Handle "pgn <file>" to feed moves from a PGN file
        {
            let mut parts = input.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            if cmd.eq_ignore_ascii_case("pgn") {
                let path = parts.next().unwrap_or("").trim();
                if path.is_empty() {
                    println!("Use command 'pgn <file>\n");
                    continue;
                }
                match game.reset() {
                    Ok(_) => {
                        match PgnPlayer::play(path, &mut game) {
                            Ok(_) => println!("PGN replay finished.\n"),
                            Err(e) => println!("Error replaying PGN: {}\n", e),
                        }
                    },
                    Err(e) => println!("Error resetting board: {}\n", e),
                }
                continue;
            }
        }

        let mut generate_move = if input.eq_ignore_ascii_case("?") { true } else {false};
        if game.active_color_is_white() {
            if bot_vs_player || bot_vs_bot { generate_move = true; }
        } else
            if player_vs_bot || bot_vs_bot { generate_move = true;
        }

        if generate_move {
            let active_color = game.get_game_state().active_color();
            let board = game.board();
            if let Some(generated_move) = generate_move_as_san(board, active_color) {
                println!("{}\n", generated_move);

                if !game.move_piece_san(generated_move.as_str()) {
                    println!("Illegal or invalid move: '{}'. Try again.\n", generated_move);
                }
            }
        } else {
            // manual move
            if !game.move_piece_san(input.as_str()) {
                println!("Illegal or invalid move: '{}'. Try again.\n", input);
            }
        }

        println!("{}", game.board());

        let score = evaluate_position(game.board());

        println!("Evaluation: {}", score as f32/100.0);

        game.get_game_state().recompute_outcome();
        if let Some(outcome) = game.get_game_state().get_outcome() {
            if outcome != OutcomeType::Ongoing && outcome != OutcomeType::InCheck {
                player_vs_player = true;
                player_vs_bot = false;
                bot_vs_bot = false;
                bot_vs_player = false;
            }
        }
    }
}







