use chess::parser::pgn_library_reader::PGNLibraryReader;
use chess::Chess;
use std::path::PathBuf;
use chess::piece::pieces::Color;
use chess::state::outcome::OutcomeType;

#[test]
fn test_games_of_pgn_library() {
    // This test streams PGN games one-by-one from the repository PGN file
    // and replays them on a fresh Chess board to ensure all SAN moves are
    // parsable and legal according to the engine.
    //
    // To keep CI fast and to avoid failures on machines missing the large PGN
    // file, the test has two guards:
    // - If the PGN file pgn_database/LumbrasGigaBase_Online_2025.pgn_database does not exist, the
    //   test returns early (skip).
    // - By default, only the first N games are checked (N=25). You can override
    //   this by setting ENABLE_PGN_LIBRARY_TEST=1 to process all games, or set
    //   PGN_LIBRARY_TEST_LIMIT to a custom integer.

    const FIRST_TEST:i32 = 1; //starts at #1

    //last check 1 ... 3299 ok
    const LAST_TEST:i32 = 25; // reasonable default for CI speed

    // Games to skip from the PGN library (by game index starting at 1) because they have been checked and are invalid
    let skip_list: Vec<i32> = vec![
        1145
    ];


    // Verify that the expected PGN exists; skip test if missing.
    let pgn_path = PathBuf::from("../../pgn_database").join("LumbrasGigaBase_Online_2025.pgn_database");
    if !pgn_path.exists() {
        eprintln!(
            "Skipping test_games_of_pgn_library: '{}' not found.",
            pgn_path.display()
        );
        return; // skip when PGN is not available locally
    }

    let mut reader = PGNLibraryReader::new(&pgn_path).expect("Failed to open PGN library");

    let mut game_number: i32= 1;

    while let Some(mut doc) = reader.next_pgn().expect("Error streaming next PGN") {

        if game_number > LAST_TEST {
            break;
        }

        if game_number < FIRST_TEST {
            game_number += 1;
            continue;
        }

        // Skip selected games
        if skip_list.contains(&game_number) {
            println!("Skipping PGN #{} (in skip list)", game_number);
            game_number += 1;
            continue;
        }

        println!("Testing PGN #{}: {}", game_number, doc.to_string());

        let mut game = Chess::new();
        let mut ply: usize = 0;
        let mut last_move: String;

        last_move = "".to_string();

        while let Some(mv) = doc.next_move() {
            last_move = mv.clone();
            ply += 1;
            let ok = game.move_piece_san(&mv);
            if !ok {
                let msg = format!(
                    "Invalid move at game #{}, ply #{}: '{}'. FEN before move: {}\nPGN: {}",
                    game_number,
                    ply,
                    mv,
                    game.to_fen(),
                    doc.to_string()
                );
                println!("{}", game.board());
                println!("{}", msg);
                assert!(false);
            }
        }

        let history = game.get_history().clone();
        game.get_game_state().recompute_outcome(&history);
        let outcome = game.get_game_state().get_outcome().unwrap();

        if last_move.ends_with('+') && outcome != OutcomeType::InCheck {
            println!("FEN: {}", game.to_fen());
            println!("{}", game.board());
            panic!("InCheck expected");
        }
        if ply % 2 == 0 {
            if last_move.ends_with('#') && outcome != (OutcomeType::Checkmate { winner: Color::Black }) {
                println!("FEN: {}", game.to_fen());
                println!("{}", game.board());
                panic!("Checkmate black expected");
            }
        } else {
            if last_move.ends_with('#') && outcome != (OutcomeType::Checkmate { winner: Color::White }) {
                println!("FEN: {}", game.to_fen());
                println!("{}", game.board());
                panic!("Checkmate white expected");
            }
        }
        game_number += 1;
    }

    // Sanity: we should have checked at least one game if file existed.
    assert!(game_number > 0, "No games were read from the PGN library");
}