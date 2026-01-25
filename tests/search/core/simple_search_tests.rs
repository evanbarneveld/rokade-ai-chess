use chess::board::san_move::convert_move_to_san;
use chess::history::history::History;
use chess::search::{find_best_move_with_mode, SearchMode};
use chess::search::SearchContext;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn test_mate_in_1_using_simple_search() {
    // Back rank mate: White rook on d1, Black king on g8, White king on g1
    // The only winning move is Rd8# (checkmate)
    let fen = "6k1/5ppp/8/8/8/8/r4PPP/3R2K1 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();

    // Depth 2 is sufficient to find mate-in-1
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Test, &gs, &history, 2,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_eq!(san_move.unwrap(), "Rd8#")
}

//#[test] //very slow test
fn _test_mate_in_2_using_simple_search() {
    /*
    Henry Buckle vs NN, London, 1840
    r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1
    1. Nf6+ gxf6 2. Bxf7#
     */
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Test, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_eq!(san_move.unwrap(), "Nf6+")
}

//#[test] // slow test
fn _test_mate_in_1_using_simple_search_2() {
    let fen = "6k1/pp4p1/2p5/2bp4/8/P5Pb/1P3r1P/2BRRNK1 b - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Test, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Rg2+"); //not the best move
    assert_eq!(san_move.unwrap(), "Rxf1#"); //best move
}


//#[test]
fn _test_mate_in_2_using_simple_search_3() {
    /*
        Luke McShane vs Aiden Leech, London, 1994
        5b2/q4r1p/p3k1p1/2pNppP1/1P6/3Q1P1P/P7/1K1R4 w - - 0 1
        1. Nf4+ exf4 2. Qe2#
     */
    let fen = "5b2/q4r1p/p3k1p1/2pNppP1/1P6/3Q1P1P/P7/1K1R4 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Test, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Re1#"); //not the best move
    assert_eq!(san_move.unwrap(), "Qe2#"); //best move
}

//#[test] //very slow test
fn _test_mate_in_2_using_simple_search_1() {
    let fen = "r1nk3r/2b2ppp/p3b3/3NN3/Q2P3q/B2B4/P4PPP/4R1K1 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Test, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Nc6+"); //not the best move
    assert_eq!(san_move.unwrap(), "Qd7+"); //best move
}

