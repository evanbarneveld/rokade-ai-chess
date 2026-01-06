use serial_test::serial;
use chess::piece::perft::{perft_count};
use chess::state::fen::reader::reset_from_fen;
use chess::Chess;

/// perft tests, see https://www.chessprogramming.org/Perft_Results
///

#[test]
#[serial]
fn perft_position2_depth4() { //last tested at 6-jan-2026
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 4), 4_085_603);
}


#[test]
#[serial]
fn perft_position2_depth5() { //last tested at 6-jan-2026 (412 seconds)
    let time_start = std::time::Instant::now();
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 5) as i64;
    let expected: i64 = 193_690_690;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    println!("elapsed time {:?}", time_start.elapsed());
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position3_depth4() { // last tested at 6-jan-2026
    let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 4), 43_238);
}

#[test]
#[serial]
fn perft_position4_depth4() { // last tested at 6-jan-2026
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 4) as i64;
    let expected: i64 = 422_333;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position4_depth5() { // last tested at 6-jan-2026
    let time_start = std::time::Instant::now();
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 5) as i64;
    let expected: i64 = 15_833_292;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    println!("elapsed time {:?}", time_start.elapsed());
    assert_eq!(nodes, expected);
}


#[test]
#[serial]
fn perft_position5_depth4() { // last tested at 6-jan-2026
    let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes: i64 = perft_count(&gs, 4) as i64;
    let expected: i64 = 2_103_487;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }

    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position6_depth4() { // last tested at 6-jan-2026
    let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 4) as i64;
    let expected: i64 = 3_894_594;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_startpos_depths_4_and_5() { // last tested at 6-jan-2026
    let fen = Chess::DEFAULT_CHESS_STARTING_FEN;
    let gs = reset_from_fen(fen).expect("valid startpos FEN");

    assert_eq!(perft_count(&gs, 4), 197_281);
    assert_eq!(perft_count(&gs, 5), 4_865_609);
}

#[test]
#[serial]
fn perft_startpos_depth_6() {
    let fen = Chess::DEFAULT_CHESS_STARTING_FEN;
    let gs = reset_from_fen(fen).expect("valid startpos FEN");

    assert_eq!(perft_count(&gs, 6), 119_060_324);
}