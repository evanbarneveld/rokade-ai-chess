use chess::parser::parser::MoveParser;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn parse_basic_pawn_move() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let mut board = *gs.board();
    let mut parser = MoveParser::new();

    let parsed = parser
        .parse(&mut board, gs.active_color(), "e4", gs.en_passant_target())
        .expect("parse");

    assert_eq!(parsed.from, (1, 4));
    assert_eq!(parsed.to, (3, 4));
}

#[test]
fn parse_castling_move() {
    let gs = reset_from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1")
        .expect("Invalid FEN");
    let mut board = *gs.board();
    let mut parser = MoveParser::new();

    let parsed = parser
        .parse(&mut board, gs.active_color(), "O-O", gs.en_passant_target())
        .expect("parse");

    assert_eq!(parsed.from, (0, 4));
    assert_eq!(parsed.to, (0, 6));
}
