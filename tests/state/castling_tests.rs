use chess::state::test_support::CastlingRights;

#[test]
fn castling_rights_parse_and_format() {
    let rights = CastlingRights::from_fen("Kq");
    assert!(rights.white_kingside());
    assert!(!rights.white_queenside());
    assert!(!rights.black_kingside());
    assert!(rights.black_queenside());
    assert_eq!(rights.to_fen(), "Kq");
}

#[test]
fn castling_rights_revoke() {
    let mut rights = CastlingRights::all();
    rights.revoke_white_castling();
    assert!(!rights.white_kingside());
    assert!(!rights.white_queenside());
}
