use chess::board::test_support::{mirror_row_for_black, pst_value_tapered, tapered_eval};
use chess::piece::pieces::{Color, PieceType};

#[test]
fn mirror_row_for_black_flips_rank() {
    assert_eq!(mirror_row_for_black(0), 7);
    assert_eq!(mirror_row_for_black(7), 0);
}

#[test]
fn tapered_eval_interpolates_between_phases() {
    let mg = 24;
    let eg = 0;
    assert_eq!(tapered_eval(mg, eg, 24), 24);
    assert_eq!(tapered_eval(mg, eg, 0), 0);
}

#[test]
fn pst_value_tapered_mirrors_for_black() {
    let white = pst_value_tapered(PieceType::Pawn, 1, 0, Color::White, 24);
    let black = pst_value_tapered(PieceType::Pawn, 6, 0, Color::Black, 24);
    assert_eq!(white, black);
}
