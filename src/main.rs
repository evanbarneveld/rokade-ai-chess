use std::io::{self, Write};
use chess::board::evaluator::evaluate_position;
use chess::Chess;
use chess::pgn_player::pgn_player::PgnPlayer;
use chess::state::outcome::OutcomeType;
use chess::generator::move_generator::generate_move_as_san;

// Represents who is playing: humans or engine
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameMode {
    PlayerVsPlayer,
    PlayerVsBot,
    BotVsPlayer,
    BotVsBot,
}

fn main() {

    let mut mode = GameMode::PlayerVsPlayer;

    println!("Welcome to chess. Type 'help' for help, enter move, or 'exit' to exit.\n");

    let mut game = Chess::new();

    if let Err(e) = game.reset() {
        eprintln!("Error resetting: {}", e);
        return;
    }

    // print the initial board to the console
    println!("{}", game.board());

    // loop for each move
    loop {
        print_user_prompt(&mut game);

        let input: Option<String>;
        let mut move_is_bot_move: bool = false;

        input = read_user_input(&mut game, &mut mode);

        if input.is_some() {
            let some_input = input.unwrap();
            if some_input.eq_ignore_ascii_case("exit") {
                println!("Bye!");
                break;
            }

            if some_input.eq_ignore_ascii_case("help") {
                print_help();
                continue;
            }

            handle_game_mode_commands(&mut mode, &some_input);

            if some_input.eq_ignore_ascii_case("fen") {
                println!("{}\n", game.to_fen());
                println!("{}", game.board());
                continue;
            }
            if some_input.eq_ignore_ascii_case("undo") {
                game.undo_move();
                println!("{}", game.board());
                continue;
            }
            if some_input.eq_ignore_ascii_case("list") {
                game.list();
                continue;
            }

            if some_input.starts_with("reset") {
                handle_reset(&mut game, some_input);
                continue;
            }

            if some_input.starts_with("pgn") {
                handle_pgn(&mut game, some_input);
                println!("{}", game.board());
                continue;
            }

            if some_input.eq_ignore_ascii_case("?") {
                move_is_bot_move = true;
            }

            // assume the input is a move
            if !move_is_bot_move {
                if !game.move_piece_san(some_input.as_str()) {
                    println!("Illegal or invalid move: '{}'. Try again.\n", some_input);
                }
            }
        }

        // handle the move
        if must_generate_move(&mut game, &mut mode, move_is_bot_move) {
            let active_color = game.get_game_state().active_color();
            let board = game.board();
            if let Some(generated_move) = generate_move_as_san(board, active_color) {
                println!("{}\n", generated_move);

                if !game.move_piece_san(generated_move.as_str()) {
                    println!("Illegal or invalid move: '{}'. Try again.\n", generated_move);
                }
            }
        }

        println!("{}", game.board());

        let score = evaluate_position(game.board());

        println!("Evaluation: {}", score as f32 / 100.0);

        game.get_game_state().recompute_outcome();
        if let Some(outcome) = game.get_game_state().get_outcome() {
            if game.get_history().current_repetition_count() >= 3 {
                mode = GameMode::PlayerVsPlayer;
                println!("3 repetitions in a row, starting over.");
            }
            if outcome != OutcomeType::Ongoing && outcome != OutcomeType::InCheck {
                mode = GameMode::PlayerVsPlayer;
            }
        }

    }

    fn handle_reset(game: &mut Chess, input: String) {
        // Handle "reset <fen>" to reinitialize the board from a FEN string
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
                game.reset();
            } else {
                match game.set_starting_fen(fen) {
                    Ok(()) => {
                        println!("Board was reset.\n");
                        println!("{}", game.board())
                    },
                    Err(e) => println!("Error parsing FEN: {}\n", e),
                }
            }
        }
    }

    fn handle_pgn(game: &mut Chess, input: String) {
        let mut parts = input.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("");
        if cmd.eq_ignore_ascii_case("pgn") {
            let path = parts.next().unwrap_or("").trim();
            if path.is_empty() {
                println!("Use command 'pgn <file>\n");
            }
            match game.reset() {
                Ok(_) => {
                    match PgnPlayer::play(path, game) {
                        Ok(_) => println!("PGN replay finished.\n"),
                        Err(e) => println!("Error replaying PGN: {}\n", e),
                    }
                },
                Err(e) => println!("Error resetting board: {}\n", e),
            }
        }
    }

    fn print_user_prompt(mut game: &mut Chess) {
        let prompt = get_user_prompt(&mut game);
        print!("{}", prompt);
        io::stdout().flush().unwrap();
    }

    fn get_user_prompt(game: &mut Chess) -> String {
        let game_outcome = game.get_game_state().get_outcome();
        let move_number = game.get_game_state().full_move_number();

        if game_outcome != Some(OutcomeType::Ongoing) && game_outcome != Some(OutcomeType::InCheck) {
            format!("Game over: {:?}\nCommand> ", game_outcome.unwrap())
        } else {
            if game_outcome == Some(OutcomeType::InCheck) { print!("Check! "); }
            if game.active_color_is_white() {
                format!("White move {} > ", move_number)
            } else {
                format!("Black move {} > ", move_number)
            }
        }
    }

    fn read_user_input(game: &mut Chess, mode: &mut GameMode) -> Option<String> {
        if (game.active_color_is_white() && matches!(mode, GameMode::PlayerVsPlayer | GameMode::PlayerVsBot)) ||
            (!game.active_color_is_white() && matches!(mode, GameMode::PlayerVsPlayer | GameMode::BotVsPlayer)) {
            let mut input_line = String::new();
            if io::stdin().read_line(&mut input_line).is_err() {
                return None
            }
            return Some(input_line.trim().to_string());
        }
        None
    }

    fn must_generate_move(game: &mut Chess, mode: &mut GameMode, move_is_bot_move : bool) -> bool {
        if game.active_color_is_white() {
            if matches!(mode, GameMode::BotVsPlayer | GameMode::BotVsBot) { return true }
        } else {
            if matches!(mode, GameMode::PlayerVsBot | GameMode::BotVsBot) { return true }
        }
        move_is_bot_move
    }

    fn print_help() {
        println!("reset          - reset the board to the initial position");
        println!("reset <fen>    - set initial position to the FEN position, and reset the board");
        println!("reset standard - set the initial position to the standard position and reset the board");
        println!("pgn <file>     - replay the given PGN file");
        println!("?              - automatic move");
        println!("fen            - print the current FEN position");
        println!("undo           - undo the last move");
        println!("list           - list all moves");
        println!("pvsb           - play vs bot");
        println!("bvsb           - bot vs bot");
        println!("bvsp           - bot vs player");
        println!("pvp            - player vs player");
        println!("exit           - exit the program");
    }

    fn handle_game_mode_commands(mode: &mut GameMode, some_input: &String) {
        if some_input.eq_ignore_ascii_case("pvsb") { *mode = GameMode::PlayerVsBot; }
        if some_input.eq_ignore_ascii_case("bvsb") { *mode = GameMode::BotVsBot; }
        if some_input.eq_ignore_ascii_case("bvsp") { *mode = GameMode::BotVsPlayer; }
        if some_input.eq_ignore_ascii_case("pvp") { *mode = GameMode::PlayerVsPlayer; }
    }

}


