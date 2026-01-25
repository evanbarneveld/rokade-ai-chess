use chess::board::evaluator::MATE_VALUE;
use chess::search::test_support::{from_tt_score, to_tt_score, MATE_TB};

#[test]
fn mate_score_round_trip_white() {
    let score = MATE_VALUE - 3;
    let ply = 5;
    let stored = to_tt_score(score, ply);
    let restored = from_tt_score(stored, ply);
    assert_eq!(restored, score);
    assert!(score > MATE_TB);
}

#[test]
fn mate_score_round_trip_black() {
    let score = -(MATE_VALUE - 4);
    let ply = 7;
    let stored = to_tt_score(score, ply);
    let restored = from_tt_score(stored, ply);
    assert_eq!(restored, score);
    assert!(score < -MATE_TB);
}

#[test]
fn mate_score_adjusts_with_ply() {
    let score = MATE_VALUE - 1;
    let stored = to_tt_score(score, 0);
    let restored = from_tt_score(stored, 2);
    assert_eq!(restored, MATE_VALUE - 3);
}

#[test]
fn losing_mate_score_adjusts_with_ply() {
    let score = -(MATE_VALUE - 1);
    let stored = to_tt_score(score, 0);
    let restored = from_tt_score(stored, 2);
    assert_eq!(restored, -(MATE_VALUE - 3));
}

#[test]
fn non_mate_score_round_trip() {
    let score = 1234;
    let ply = 9;
    let stored = to_tt_score(score, ply);
    let restored = from_tt_score(stored, ply);
    assert_eq!(restored, score);
}

#[test]
fn mate_score_order_preserved() {
    let m1 = MATE_VALUE - 1;
    let m3 = MATE_VALUE - 3;
    let s1 = to_tt_score(m1, 2);
    let s3 = to_tt_score(m3, 2);
    assert!(s1 > s3);
}

#[test]
fn losing_mate_order_preserved() {
    let m1 = -(MATE_VALUE - 1);
    let m3 = -(MATE_VALUE - 3);
    let s1 = to_tt_score(m1, 2);
    let s3 = to_tt_score(m3, 2);
    assert!(s1 < s3);
}
