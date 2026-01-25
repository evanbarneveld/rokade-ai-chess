use chess::pgn_player::pgn_player::PgnPlayer;
use chess::Chess;
use std::fs;

#[test]
fn pgn_player_replays_simple_game() {
    let pgn = "1. e4 e5 2. Nf3 Nc6 *";
    let path = std::env::temp_dir().join("rokade_ai_test.pgn");
    fs::write(&path, pgn).expect("write pgn");

    let mut game = Chess::new();
    let result = PgnPlayer::play(path.to_str().unwrap(), &mut game);
    assert!(result.is_ok());
    assert_eq!(game.get_history().len(), 4);
}
