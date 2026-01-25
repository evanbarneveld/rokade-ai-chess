use chess::piece::piece_movers::move_pawn::move_pawn;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn move_pawn_sets_en_passant_target_on_double_push() {
    let mut gs = reset_from_fen("4k3/8/8/8/8/8/P7/4K3 w - - 0 1")
        .expect("Invalid FEN");
    let pawn = Piece::new(PieceType::Pawn, Color::White);

    let ok = move_pawn(&mut gs, pawn, (1, 0), (3, 0), false, None);
    assert!(ok);
    assert_eq!(gs.en_passant_target(), Some((2, 0)));
    assert_eq!(gs.half_move_clock(), 0);
}

#[test]
fn move_pawn_auto_promotes_when_no_piece_provided() {
    let mut gs = reset_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1")
        .expect("Invalid FEN");
    let pawn = Piece::new(PieceType::Pawn, Color::White);

    let ok = move_pawn(&mut gs, pawn, (6, 0), (7, 0), false, None);
    assert!(ok);
    let promoted = gs.board().get(7, 0).expect("promotion");
    assert_eq!(promoted.get_type(), PieceType::Queen);
}
