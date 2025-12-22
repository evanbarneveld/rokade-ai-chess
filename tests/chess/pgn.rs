use std::path::PathBuf;
use chess::parser::pgn_document::PGNDocument;

#[test]
fn read_simple_pgn_and_iterate_moves() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("data");
    path.push("simple.pgn");

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
