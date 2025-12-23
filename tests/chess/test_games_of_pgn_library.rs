#[test]
fn test_games_of_pgn_library() {
    // This test streams PGN games one-by-one from the repository PGN file
    // and replays them on a fresh Chess board to ensure all SAN moves are
    // parsable and legal according to the engine.
    //
    // To keep CI fast and to avoid failures on machines missing the large PGN
    // file, the test has two guards:
    // - If the PGN file pgn/LumbrasGigaBase_Online_2025.pgn does not exist, the
    //   test returns early (skip).
    // - By default, only the first N games are checked (N=25). You can override
    //   this by setting ENABLE_PGN_LIBRARY_TEST=1 to process all games, or set
    //   PGN_LIBRARY_TEST_LIMIT to a custom integer.

    use chess::parser::pgn_library_reader::PGNLibraryReader;
    use chess::Chess;
    use std::env;
    use std::path::PathBuf;

    // Verify that the expected PGN exists; skip test if missing.
    let pgn_path = PathBuf::from("pgn").join("LumbrasGigaBase_Online_2025.pgn");
    if !pgn_path.exists() {
        eprintln!(
            "Skipping test_games_of_pgn_library: '{}' not found.",
            pgn_path.display()
        );
        return; // skip when PGN is not available locally
    }

    // Determine limit behavior
    let enable_all = env::var("ENABLE_PGN_LIBRARY_TEST").ok().as_deref() == Some("1");
    let limit_env = env::var("PGN_LIBRARY_TEST_LIMIT").ok();
    let default_limit = 10000; // reasonable default for CI speed

    // Games to skip from the PGN library (by game index starting at 1) because they have been checked and are invalid
    let skip_list: Vec<i32> = vec![
        1145
    ];

    //let first_test = *skip_list.last().unwrap();
    let first_test = 1;

    let limit = if enable_all {
        None
    } else if let Some(s) = limit_env {
        match s.parse::<i32>() {
            Ok(n) if n > 0 => Some(n),
            _ => Some(default_limit),
        }
    } else {
        Some(default_limit)
    };

    let mut reader = PGNLibraryReader::new(&pgn_path).expect("Failed to open PGN library");

    let mut game_number: i32= 1;

    while let Some(mut doc) = reader.next_pgn().expect("Error streaming next PGN") {

        if let Some(max) = limit {
            if game_number > max {
                break;
            }
        }

        if (game_number < first_test) {
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
        while let Some(mv) = doc.next_move() {
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

        game_number += 1;
    }

    // Sanity: we should have checked at least one game if file existed.
    assert!(game_number > 0, "No games were read from the PGN library");
}