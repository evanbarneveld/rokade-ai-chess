use std::path::PathBuf;
use chess::parser::pgn_document::PGNDocument;

#[test]
fn read_simple_pgn_and_iterate_moves() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("data");
    path.push("simple.pgn_database");

    let mut doc = PGNDocument::from_file(&path).expect("should load PGN");

    let expected = vec!["e4", "e5", "Nf3", "Nc6", "Bb5", "a6"]; 
    for exp in expected.iter() {
        let mv = doc.next_move();
        assert_eq!(mv.as_deref(), Some(*exp));
    }
    assert_eq!(doc.next_move(), None);

    // reset should allow iterating again
    doc.reset();
    assert_eq!(doc.next_move().as_deref(), Some("e4"));
}

#[test]
fn read_pgn_and_iterate_moves() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("data");
    path.push("with_timers.pgn_database");

    let mut doc = PGNDocument::from_file(&path).expect("should load PGN");

    let expected = vec!["Nc3", "d5", "e4", "dxe4", "Nxe4", "Nd7"];
    for exp in expected.iter() {
        let mv = doc.next_move();
        assert_eq!(mv.as_deref(), Some(*exp));
    }
    assert_eq!(doc.next_move(), None);

    // reset should allow iterating again
    doc.reset();
    assert_eq!(doc.next_move().as_deref(), Some("Nc3"));
}

//1. Nc3 {[%clk 1:30:55]} 1. ... d5 {[%clk 1:30:54]} 2. e4 {[%clk 1:31:23]}
// 2. ... dxe4 3. Nxe4 {[%clk 1:31:50]} 3. ... Nd7 {[%clk 1:31:06]}
