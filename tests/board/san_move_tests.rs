use chess::board::san_move::convert_move_to_san;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn convert_move_to_san_formats_pawn_moves() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let san = convert_move_to_san(&gs, Some(((1, 4), (3, 4), None))).expect("san");
    assert_eq!(san, "e4");
}
