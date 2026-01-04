use serial_test::serial;
use chess::piece::perft::{perft_count, perft_divide};
use chess::state::fen::reader::reset_from_fen;
use chess::Chess;
use chess::search::advanced_search::_dump_all_valid_moves;

/// perft tests, see https://www.chessprogramming.org/Perft_Results
///

#[test]
#[serial]
fn perft_position2_depth1() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 1), 48);
}

#[test]
#[serial]
fn perft_position2_depth2() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 2), 2039);
}

#[test]
#[serial]
fn perft_position2_depth3() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 3), 97_862);
}

#[test]
#[serial]
fn perft_position3_depth1() {
    let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 1), 14);
}

#[test]
#[serial]
fn perft_position3_depth2() {
    let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    // Dump all valid moves for debugging this perft position
    _dump_all_valid_moves(&gs, false);
    assert_eq!(perft_count(&gs, 2), 191);
}

#[test]
#[serial]
fn perft_position3_depth3() {
    let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 3), 2812);
}

#[test]
#[serial]
fn perft_position4_depth1() {
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    assert_eq!(perft_count(&gs, 1), 6);
}

#[test]
#[serial]
fn perft_position4_depth2() {
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 2) as i64;
    let expected: i64 = 264;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position4_depth3() {
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 3) as i64;
    let expected: i64 = 9467;

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position5_depth1() {
    let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 1) as i64;
    let expected: i64 = 44; //44, 1486, 62379

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }

    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position5_depth2() {
    let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes: i64 = perft_count(&gs, 2) as i64;
    let expected: i64 = 1486; //44, 1486, 62379

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }

    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position5_depth3() {
    let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes: i64 = perft_count(&gs, 3) as i64;
    let expected: i64 = 62_379; //44, 1486, 62379

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }

    assert_eq!(nodes, expected);
}


#[test]
#[serial]
fn perft_position6_depth1() {
    let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 1) as i64;
    let expected: i64 = 46; //46, 2079, 89890

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position6_depth2() {
    let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 2) as i64;
    let expected: i64 = 2079; //46, 2079, 89890

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_position6_depth3() {
    let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let nodes:i64 = perft_count(&gs, 3) as i64;
    let expected: i64 = 89890; //46, 2079, 89890

    if nodes - expected != 0 {
        println!("diff = {}", (nodes - expected).abs());
    }
    assert_eq!(nodes, expected);
}

#[test]
#[serial]
fn perft_startpos_depths() {
    let fen = Chess::DEFAULT_CHESS_STARTING_FEN;
    let gs = reset_from_fen(fen).expect("valid startpos FEN");

    assert_eq!(perft_count(&gs, 1), 20);
    assert_eq!(perft_count(&gs, 2), 400);
    assert_eq!(perft_count(&gs, 3), 8_902);
}
