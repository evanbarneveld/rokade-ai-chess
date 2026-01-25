use chess::parser::pgn_document::PGNDocument;

#[test]
fn pgn_document_parses_moves_and_skips_headers() {
    let pgn = r#"
[Event "Test"]
[Site "Local"]
1. e4 e5 {comment} 2. Nf3 Nc6 1-0
"#;
    let doc = PGNDocument::from_str(pgn);
    assert_eq!(doc.len(), 4);
    assert_eq!(doc.clone().next_move().unwrap(), "e4");
}

#[test]
fn pgn_document_display_emits_move_text() {
    let pgn = "1. d4 d5 2. c4 e6 *";
    let doc = PGNDocument::from_str(pgn);
    let output = doc.to_string();
    assert!(output.contains("1. d4 d5 2. c4 e6"));
}
