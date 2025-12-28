use std::io;
use std::io::Write;
use crate::board::evaluator::evaluate_position;
use crate::Chess;
use crate::generator::move_generator::generate_move_as_san;
use crate::pgn_player::pgn_player::PgnPlayer;
use crate::state::outcome::OutcomeType;
use crate::uci::run_uci;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameMode {
    PlayerVsPlayer,
    PlayerVsBot,
    BotVsPlayer,
    BotVsBot,
}

pub fn run_cli() {
    let mut mode = GameMode::PlayerVsPlayer;
    let mut white_bot_strength: usize = 1000;
    let mut black_bot_strength: usize = 1000;

    println!("Welcome to chess. Type 'help' for help, enter move, or 'exit' to exit.\n");

    let mut game = Chess::new();

    if let Err(e) = game.reset() {
        eprintln!("Error resetting: {}", e);
        return;
    }

    // loop for each move
    loop {

        let history = game.get_history().clone();
        println!("{}", game.board().get_board_display_string(&history));

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

            if some_input.eq_ignore_ascii_case("uci") {
                println!("id name eriks-chess");
                println!("id author erik van barneveld");
                println!("uciok");
                io::stdout().flush().unwrap();
                run_uci().unwrap();
                break;
            }

            if handle_game_mode_commands(&mut mode, &some_input) {
                continue;
            }


            if some_input.eq_ignore_ascii_case("fen") {
                println!("{}\n", game.to_fen());
                continue;
            }
            if some_input.eq_ignore_ascii_case("undo") {
                game.undo_move();
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

            if some_input.starts_with("pgn_database") {
                handle_pgn(&mut game, some_input);
                continue;
            }

            if some_input.starts_with("strength") {
                if handle_strength(some_input.clone(), &mut white_bot_strength, &mut black_bot_strength) {
                    continue
                }
            }

            if some_input.eq_ignore_ascii_case("eval") {
                let score = evaluate_position(game.board());
                println ! ("Evaluation: {}", score as f32 / 100.0);
                continue;
            }

            if some_input.eq_ignore_ascii_case("?") {
                move_is_bot_move = true;
            }

            // assume the input is a move
            if !move_is_bot_move && !some_input.is_empty() {
                if !game.move_piece_san(some_input.as_str()) {
                    println!("Illegal or invalid move: '{}'. Try again.\n", some_input);
                }
            }
        }

        // handle the move
        if must_generate_move(&mut game, &mut mode, move_is_bot_move) {
            let history = game.get_history().clone();
            let strength = if game.active_color_is_white() { white_bot_strength } else { black_bot_strength };
            if let Some(generated_move) = generate_move_as_san(*game.get_game_state(), history, strength) {
                println ! ("{}\n", generated_move);

                if !game.move_piece_san(generated_move.as_str()) {
                    println ! ("Illegal or invalid move: '{}'. Try again.\n", generated_move);
                }
            } else {
                println ! ("No legal moves available.\n");
            }
        }

        let history = game.get_history().clone();
        game.get_game_state().recompute_outcome(&history);

        if let Some(outcome) = game.get_game_state().get_outcome() {
            if outcome != OutcomeType::Ongoing && outcome != OutcomeType::InCheck {
                mode = GameMode::PlayerVsPlayer;
            }
        }

    }
}

fn handle_strength(input: String, white_bot_strength: &mut usize, black_bot_strength: &mut usize) -> bool {

    let mut parts = input.split_whitespace();
    let _ = parts.next(); // consume 'strength'
    let first = parts.next();

    if first.is_none() {
        println!("White bot strength {}, black bot strength {}", white_bot_strength, black_bot_strength);
        return true
    }
    let second = parts.next();

    // helper to parse a usize in 0..=1000
    let parse_strength = |s: &str| -> Option<usize> {
        match s.parse::<usize>() {
            Ok(v) if v <= 1000 => Some(v),
            _ => None
        }
    };

    match (first, second) {
        // strength <n>  -> set both
        (Some(n), None) => {
            if let Some(v) = parse_strength(n) {
                *white_bot_strength = v;
                *black_bot_strength = v;
                println!("Set both bot strengths to {} (0..1000).", v);
            } else {
                println!("Invalid strength. Use: strength <0..1000> | strength white <0..1000> | strength black <0..1000>");
            }
            return true
        }
        // strength white <n>
        (Some(color), Some(n)) if color.eq_ignore_ascii_case("white") => {
            if let Some(v) = parse_strength(n) {
                *white_bot_strength = v;
                println!("Set white bot strength to {} (0..1000).", v);
            } else {
                println!("Invalid strength for white. Use: strength white <0..1000>");
            }
            return true
        }
        // strength black <n>
        (Some(color), Some(n)) if color.eq_ignore_ascii_case("black") => {
            if let Some(v) = parse_strength(n) {
                *black_bot_strength = v;
                println!("Set black bot strength to {} (0..1000).", v);
            } else {
                println!("Invalid strength for black. Use: strength black <0..1000>");
            }
            return true
        }
        _ => {
            println!("Usage: strength <0..1000> | strength white <0..1000> | strength black <0..1000>");
            return true
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
                },
                Err(e) => println!("Error resetting board: {}\n", e),
            }
        } else if fen.eq("standard") {
            let _ = game.reset();
        } else {
            match game.set_starting_fen(fen) {
                Ok(()) => {
                    println!("Board was reset.\n");
                },
                Err(e) => println!("Error parsing FEN: {}\n", e),
            }
        }
    }
}

fn handle_pgn(game: &mut Chess, input: String) {
    let mut parts = input.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    if cmd.eq_ignore_ascii_case("pgn_database") {
        let path = parts.next().unwrap_or("").trim();
        if path.is_empty() {
            println!("Use command 'pgn_database <file>\n");
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
    println!("pgn_database <file>     - replay the given PGN file");
    println!("?              - automatic move");
    println!("fen            - print the current FEN position");
    println!("undo           - undo the last move");
    println!("list           - list all moves");
    println!("strength       - get the strength of the bots (higher = stronger)");
    println!("strength <0..1000>            - set both bots' playing strength");
    println!("strength white <0..1000>      - set white bot playing strength");
    println!("strength black <0..1000>      - set black bot playing strength");
    println!("pvsb           - play vs bot");
    println!("bvsb           - bot vs bot");
    println!("bvsp           - bot vs player");
    println!("pvp            - player vs player");
    println!("exit           - exit the program");
}

fn handle_game_mode_commands(mode: &mut GameMode, some_input: &String) -> bool {
    if some_input.eq_ignore_ascii_case("pvsb") { *mode = GameMode::PlayerVsBot; return true};
    if some_input.eq_ignore_ascii_case("bvsb") { *mode = GameMode::BotVsBot; return true};
    if some_input.eq_ignore_ascii_case("bvsp") { *mode = GameMode::BotVsPlayer; return true };
    if some_input.eq_ignore_ascii_case("pvp") { *mode = GameMode::PlayerVsPlayer; return true};
    false
}
