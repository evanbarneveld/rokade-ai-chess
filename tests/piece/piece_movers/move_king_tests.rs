use chess::piece::piece_movers::move_king::move_king;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn move_king_handles_castling_rook_move() {
    let mut gs = reset_from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1")
        .expect("Invalid FEN");
    let king = Piece::new(PieceType::King, Color::White);

    let ok = move_king(&mut gs, king, (0, 4), (0, 6), false);
    assert!(ok);
    assert!(gs.board().get(0, 5).is_some());
    assert!(gs.board().get(0, 7).is_none());
    assert!(!gs.castling_rights().white_kingside());
    assert_eq!(gs.half_move_clock(), 1);
}
