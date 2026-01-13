use std::sync::atomic::{AtomicBool, Ordering};
use crate::piece::pieces::{PieceType};
use crate::state::game_state::GameState;
use crate::search::core::advanced_search::find_all_valid_moves;
use rand::{rng, Rng};

static ORDER_BOOK_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn get_order_book_enabled() -> bool {
    ORDER_BOOK_ENABLED.load(Ordering::Relaxed)
}

pub fn set_order_book_enabled(enabled: bool) {
    ORDER_BOOK_ENABLED.store(enabled, Ordering::Relaxed);
}

// A tiny in-code opening book keyed by truncated FEN (first 4 fields).
// Values are (UCI move string, weight).
static BOOK: &[(&str, &[(&str, u32)])] = &[
    // Initial position
    (
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -",
        &[("e2e4", 40), ("d2d4", 30), ("c2c4", 15), ("g1f3", 15)],
    ),
    // 1... e5
    (
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("g1f3", 60), ("f1c4", 20), ("d2d4", 20)],
    ),
    // 1... c5 (Sicilian)
    (
        "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("g1f3", 50), ("d2d4", 30), ("c2c3", 20)],
    ),
    // 1... e6 (French)
    (
        "rnbqkbnr/pppppppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("d2d4", 60), ("g1f3", 40)],
    ),
    // 1... c6 (Caro-Kann)
    (
        "rnbqkbnr/pppppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("d2d4", 60), ("g1f3", 40)],
    ),
    // After 1.d4 ... d5
    (
        "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &[("c1f4", 30), ("g1f3", 40), ("c2c4", 30)],
    ),
    // Black replies vs 1.e4: ...e5, ...c5, ...e6
    (
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq -",
        &[("e7e5", 40), ("c7c5", 35), ("e7e6", 25)],
    ),
    // Black replies vs 1.d4: ...d5, ...Nf6
    (
        "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq -",
        &[("d7d5", 55), ("g8f6", 45)],
    ),

    // 1.c4 (English) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq -",
        &[("e7e5", 30), ("c7c5", 30), ("g8f6", 25), ("e7e6", 15)],
    ),

    // 1.Nf3 (Zukertort/Reti setups) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq -",
        &[("d7d5", 30), ("g8f6", 40), ("c7c5", 15), ("e7e6", 15)],
    ),

    // 1.e4 e5 2.Nf3 — Common Black replies
    (
        "rnbqkbnr/pppppppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -",
        &[("b8c6", 50), ("g8f6", 30), ("d7d6", 20)],
    ),

    // 1.e4 c5 2.Nf3 — Common Black replies (Sicilian branches)
    (
        "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -",
        &[("d7d6", 40), ("b8c6", 35), ("e7e6", 15), ("g7g6", 10)],
    ),

    // 1.d4 Nf6 — White main continuations
    (
        "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &[("c2c4", 50), ("g1f3", 30), ("b1c3", 20)],
    ),

    // 1.d4 d5 2.c4 — Black replies (QGD/Slav/Tarrasch)
    (
        "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq -",
        &[("e7e6", 45), ("c7c6", 35), ("d5c4", 20)],
    ),

    // 1.e4 e6 2.d4 d5 — White choices vs French
    (
        "rnbqkbnr/pppp1ppp/4p3/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq -",
        &[("b1c3", 40), ("b1d2", 20), ("e4e5", 20), ("c1g5", 20)],
    ),

    // 1.e4 c6 2.d4 d5 — White choices vs Caro-Kann
    (
        "rnbqkbnr/pp2pppp/2p5/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq -",
        &[("b1c3", 40), ("e4e5", 20), ("b1d2", 20), ("e4d5", 20)],
    ),

    // 1.e4 c5 2.Nf3 d6 3.d4 — Open Sicilian entry
    (
        "rnbqkb1r/pp2pppp/3p1n2/2p5/3PP3/5N2/PPP2PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 70), ("b1c3", 30)],
    ),
    // After 1.e4 c5 2.Nf3 Nc6 3.d4 — Sveshnikov/ Classical
    (
        "r1bqkbnr/pp1ppppp/2n5/2p5/3PP3/5N2/PPP2PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 70), ("b1c3", 30)],
    ),
    // 1.d4 Nf6 2.c4 e6 3.Nc3 — Black Nimzo/Bogo/Queens-Indian setups
    (
        "rnbqkb1r/pppppppp/4pn2/8/2PP4/2N5/PP2PPPP/R1BQKBNR b KQkq -",
        &[("f8b4", 40), ("b7b6", 30), ("d7d5", 30)],
    ),
    // 1.d4 d5 2.c4 e6 3.Nc3 — QGD structures
    (
        "rnbqkbnr/ppp1pppp/4p3/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq -",
        &[("g1f3", 40), ("c1f4", 30), ("c1g5", 30)],
    ),
    // Scotch Game: 1.e4 e5 2.Nf3 Nc6 3.d4
    (
        "r1bqkbnr/pppp1ppp/2n5/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq -",
        &[("e5d4", 60), ("g8f6", 25), ("d7d6", 15)],
    ),
    // Italian: 1.e4 e5 2.Nf3 Nc6 3.Bc4
    (
        "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq -",
        &[("g8f6", 55), ("f8c5", 35), ("d7d6", 10)],
    ),
    // Two Knights: 1.e4 e5 2.Nf3 Nc6 3.Bc4 Nf6
    (
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq -",
        &[("d2d4", 35), ("e1g1", 30), ("b1c3", 35)],
    ),
    // Ruy Lopez: 1.e4 e5 2.Nf3 Nc6 3.Bb5
    (
        "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq -",
        &[("a7a6", 55), ("g8f6", 25), ("f8c5", 20)],
    ),
    // Petrov: 1.e4 e5 2.Nf3 Nf6
    (
        "rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 40), ("b1c3", 30), ("f3e5", 30)],
    ),
    // Pirc/Modern: 1.e4 d6
    (
        "rnbqkbnr/ppp1pppp/3p4/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("d2d4", 50), ("g1f3", 30), ("c2c4", 20)],
    ),
    // Alekhine: 1.e4 Nf6
    (
        "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("e4e5", 50), ("b1c3", 25), ("d2d4", 25)],
    ),
    // Scandinavian: 1.e4 d5
    (
        "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &[("e4d5", 60), ("b1c3", 20), ("d2d4", 20)],
    ),
    // Vienna: 1.e4 e5 2.Nc3
    (
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/2N5/PPPP1PPP/R1BQKBNR b KQkq -",
        &[("g8f6", 45), ("b8c6", 35), ("f8c5", 20)],
    ),
    // King's Gambit: 1.e4 e5 2.f4
    (
        "rnbqkbnr/pppp1ppp/8/4p3/5P2/8/PPPPP1PP/RNBQKBNR b KQkq -",
        &[("e5f4", 55), ("d7d5", 25), ("b8c6", 20)],
    ),
    // Caro-Kann Advance: 1.e4 c6 2.d4 d5 3.e5
    (
        "rnbqkbnr/pp2pppp/2p5/3p4/3PP3/8/PPP2PPP/RNBQKBNR b KQkq -",
        &[("c8f5", 45), ("c8g4", 25), ("e7e6", 30)],
    ),
    // French Advance: 1.e4 e6 2.d4 d5 3.e5
    (
        "rnbqkbnr/pppp1ppp/4p3/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq -",
        &[("e4e5", 40), ("b1d2", 30), ("b1c3", 30)],
    ),
    (
        "rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPP2PPP/RNBQKBNR b KQkq -",
        &[("c7c5", 40), ("b8c6", 35), ("f8d6", 25)],
    ),
    // French Tarrasch: 1.e4 e6 2.d4 d5 3.Nd2
    (
        "rnbqkbnr/pppp1ppp/4p3/3p4/3PP3/8/PPPN1PPP/RNBQKBNR b KQkq -",
        &[("c7c5", 45), ("g8f6", 30), ("d5e4", 25)],
    ),
    // Slav: 1.d4 d5 2.c4 c6
    (
        "rnbqkbnr/pp2pppp/2p5/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq -",
        &[("g1f3", 40), ("b1c3", 35), ("e2e3", 25)],
    ),
    // QGD: 1.d4 d5 2.c4 e6 3.Nc3 Nf6
    (
        "rnbqkb1r/ppp1pppp/4pn2/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq -",
        &[("c1g5", 35), ("g1f3", 35), ("c1f4", 30)],
    ),
    // Semi-Slav: 1.d4 d5 2.c4 e6 3.Nc3 c6
    (
        "rnbqkbnr/pp3ppp/2p1p3/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq -",
        &[("g1f3", 40), ("e2e3", 35), ("c1f4", 25)],
    ),
    // Benoni ideas: 1.d4 Nf6 2.c4 c5 3.d5
    (
        "rnbqkb1r/pppppppp/5n2/2pP4/2P5/8/PP2PPPP/RNBQKBNR b KQkq -",
        &[("e7e6", 45), ("b7b5", 25), ("g7g6", 30)],
    ),
    // King's Indian: 1.d4 Nf6 2.c4 g6
    (
        "rnbqkb1r/pppppp1p/6p1/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq -",
        &[("g1f3", 40), ("b1c3", 35), ("e2e4", 25)],
    ),
    // Grunfeld: 1.d4 Nf6 2.c4 g6 3.Nc3 d5
    (
        "rnbqkb1r/ppp1pp1p/5np1/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq -",
        &[("c4d5", 40), ("g1f3", 35), ("e2e3", 25)],
    ),
    // English symmetrical: 1.c4 c5
    (
        "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR w KQkq -",
        &[("g1f3", 40), ("b1c3", 35), ("g2g3", 25)],
    ),
    // English ...e5 line
    (
        "rnbqkbnr/pppppppp/8/4p3/2P5/8/PP1PPPPP/RNBQKBNR w KQkq -",
        &[("b1c3", 40), ("g2g3", 30), ("g1f3", 30)],
    ),
    // Reti 1.Nf3 d5 2.c4
    (
        "rnbqkbnr/pppppppp/8/3p4/2P5/5N2/PP1PPPPP/RNBQKB1R b KQkq -",
        &[("e7e6", 40), ("c7c6", 35), ("d5c4", 25)],
    ),
    // Catalan setup: 1.d4 Nf6 2.c4 e6 3.g3
    (
        "rnbqkb1r/pppppppp/4pn2/8/2PP4/6P1/PP2PP1P/R1BQKBNR b KQkq -",
        &[("d7d5", 40), ("f8e7", 30), ("c7c5", 30)],
    ),
    // London System: 1.d4 d5 2.Nf3 Nf6 3.Bf4
    (
        "rnbqkb1r/pppppppp/5n2/3p4/3P1B2/5N2/PPP1PPPP/RN1QKB1R b KQkq -",
        &[("c7c5", 35), ("e7e6", 35), ("c8f5", 30)],
    ),
    // Trompowsky: 1.d4 Nf6 2.Bg5
    (
        "rnbqkb1r/pppppppp/5n2/6B1/3P4/8/PPP1PPPP/RN1QKBNR b KQkq -",
        &[("e7e6", 40), ("d7d5", 35), ("c7c5", 25)],
    ),
    // Queen's Gambit Accepted: 1.d4 d5 2.c4 dxc4
    (
        "rnbqkbnr/ppp1pppp/8/3p4/2pP4/8/PP2PPPP/RNBQKBNR w KQkq -",
        &[("e2e3", 40), ("g1f3", 35), ("b1c3", 25)],
    ),
    // Najdorf setup: 1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6
    (
        "rnbqkb1r/1pp1pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq -",
        &[("f1e2", 35), ("c1e3", 30), ("f2f3", 35)],
    ),
    // Dragon setup: 1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 g6
    (
        "rnbqkbnr/ppp1pp1p/3p2p1/8/3NP3/8/PPP2PPP/R1BQKBNR w KQkq -",
        &[("c2c4", 35), ("f1e2", 35), ("b1c3", 30)],
    ),
    // Classical Sicilian: 1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 d6
    (
        "r1bqkb1r/pp2pppp/3p1n2/8/3NP3/8/PPP2PPP/R1BQKBNR w KQkq -",
        &[("b1c3", 40), ("f1e2", 35), ("c1e3", 25)],
    ),
    // Carlsen's ...a6 vs English: 1.c4 e5 2.Nc3 Nf6 3.Nf3 a6
    (
        "rnbqkb1r/1ppppppp/p4n2/4p3/2P5/2N2N2/PP1PPPPP/R1BQKB1R w KQkq -",
        &[("g2g3", 40), ("e2e3", 30), ("d2d3", 30)],
    ),
    // Colle setup: 1.d4 d5 2.Nf3 e6 3.e3
    (
        "rnbqkbnr/pppp1ppp/4p3/3p4/3P4/4P3/PPP2PPP/RNBQKBNR w KQkq -",
        &[("c2c4", 35), ("b1d2", 35), ("f1d3", 30)],
    ),
    // Dutch setup: 1.d4 f5
    (
        "rnbqkbnr/pppppppp/8/5p2/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &[("c2c4", 40), ("g2g3", 35), ("b1c3", 25)],
    ),
    // Benko idea: 1.d4 Nf6 2.c4 c5 3.d5 b5
    (
        "rnbqkb1r/p1pppppp/5n2/1p1P4/2P5/8/PP2PPPP/RNBQKBNR w KQkq -",
        &[("c4b5", 40), ("a2a4", 30), ("g1f3", 30)],
    ),
    // Philidor: 1.e4 e5 2.Nf3 d6
    (
        "rnbqkbnr/pppp1ppp/3p4/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 45), ("b1c3", 35), ("f1c4", 20)],
    ),
    // Scotch Four Knights: 1.e4 e5 2.Nf3 Nc6 3.Nc3 Nf6 4.d4
    (
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/3PP3/2N2N2/PPP2PPP/R1BQKB1R b KQkq -",
        &[("e5d4", 55), ("f8b4", 25), ("d7d6", 20)],
    ),
    // Queen's Indian idea after 1.d4 Nf6 2.c4 e6 3.Nf3 b6
    (
        "rnbqkb1r/p1pppppp/1p2pn2/8/2PP4/5N2/PP2PPPP/R1BQKB1R w KQkq -",
        &[("a2a3", 35), ("g2g3", 35), ("e2e3", 30)],
    ),

    // 1.g3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/6P1/PPPPPP1P/RNBQKBNR b KQkq -",
        &[("d7d5", 30), ("e7e5", 30), ("g7g6", 25), ("g8f6", 15)],
    ),
    // 1.b3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/1P6/P1PPPPPP/RNBQKBNR b KQkq -",
        &[("e7e5", 35), ("d7d5", 35), ("g8f6", 30)],
    ),
    // 1.Nc3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/2N5/PPPPPPPP/R1BQKBNR b KQkq -",
        &[("d7d5", 35), ("e7e5", 35), ("g8f6", 30)],
    ),
    // 1.f4 (Bird) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/5P2/8/PPPPP1PP/RNBQKBNR b KQkq -",
        &[("d7d5", 35), ("g8f6", 30), ("g7g6", 20), ("e7e5", 15)],
    ),
    // 1.c3 (Saragossa) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/2P5/PP1PPPPP/RNBQKBNR b KQkq -",
        &[("d7d5", 40), ("e7e5", 35), ("g8f6", 25)],
    ),
    // 1.d3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/3P4/PPP1PPPP/RNBQKBNR b KQkq -",
        &[("d7d5", 40), ("e7e5", 35), ("g8f6", 25)],
    ),
    // 1.e3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/4P3/PPPP1PPP/RNBQKBNR b KQkq -",
        &[("d7d5", 40), ("e7e5", 35), ("g8f6", 25)],
    ),
    // 1.b4 (Polish) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/1P6/8/P1PPPPPP/RNBQKBNR b KQkq -",
        &[("e7e5", 30), ("d7d5", 30), ("g8f6", 20), ("c7c5", 20)],
    ),
    // 1.g4 (Grob) — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/6P1/8/PPPPPP1P/RNBQKBNR b KQkq -",
        &[("d7d5", 35), ("e7e5", 35), ("h7h5", 30)],
    ),
    // 1.h3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/7P/PPPPPPP1/RNBQKBNR b KQkq -",
        &[("d7d5", 40), ("e7e5", 35), ("g8f6", 25)],
    ),
    // 1.a3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/8/P7/1PPPPPPP/RNBQKBNR b KQkq -",
        &[("d7d5", 40), ("e7e5", 35), ("g8f6", 25)],
    ),
    // 1.h4 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/7P/8/PPPPPPP1/RNBQKBNR b KQkq -",
        &[("d7d5", 35), ("e7e5", 35), ("h7h5", 30)],
    ),
    // 1.a4 — Black replies
    (
        "rnbqkbnr/pppppppp/8/8/P7/8/1PPPPPPP/RNBQKBNR b KQkq -",
        &[("e7e5", 35), ("d7d5", 35), ("c7c5", 30)],
    ),
    // After 1.c4 e5 — White choices
    (
        "rnbqkbnr/pppppppp/8/4p3/2P5/8/PP1PPPPP/RNBQKBNR w KQkq -",
        &[("b1c3", 40), ("g2g3", 35), ("e2e3", 25)],
    ),
    // After 1.c4 c5 — White choices
    (
        "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR w KQkq -",
        &[("b1c3", 40), ("g2g3", 35), ("g1f3", 25)],
    ),
    // After 1.Nf3 d5 — White choices
    (
        "rnbqkbnr/pppppppp/8/3p4/8/5N2/PPPPPPPP/RNBQKB1R w KQkq -",
        &[("d2d4", 45), ("c2c4", 35), ("g2g3", 20)],
    ),
    // After 1.Nf3 Nf6 — White choices
    (
        "rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq -",
        &[("d2d4", 40), ("c2c4", 35), ("g2g3", 25)],
    ),
    // After 1.Nf3 c5 — White choices
    (
        "rnbqkbnr/pp1ppppp/8/2p5/8/5N2/PPPPPPPP/RNBQKB1R w KQkq -",
        &[("c2c4", 45), ("e2e3", 30), ("g2g3", 25)],
    ),
    // After 1.d4 Nf6 — White choices
    (
        "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &[("c2c4", 45), ("g1f3", 35), ("c1f4", 20)],
    ),
    // After 1.d4 d5 2.Nf3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R b KQkq -",
        &[("g8f6", 45), ("e7e6", 35), ("c7c5", 20)],
    ),
    // After 1.d4 d5 2.Nc3 — Black replies
    (
        "rnbqkbnr/pppppppp/8/3p4/3P4/2N5/PPP1PPPP/R1BQKBNR b KQkq -",
        &[("g8f6", 40), ("e7e6", 35), ("c7c5", 25)],
    ),
    // After 1.e4 c5 2.Nf3 e6 — White choices
    (
        "rnbqkbnr/pp2pppp/4p3/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 60), ("b1c3", 25), ("c2c3", 15)],
    ),
    // After 1.e4 c5 2.Nf3 g6 — White choices
    (
        "rnbqkbnr/pppppp1p/6p1/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -",
        &[("d2d4", 55), ("c2c3", 25), ("b1c3", 20)],
    ),
];

#[inline]
fn algebraic_to_idx(file: u8, rank: u8) -> (usize, usize) {
    // file 'a'..'h' -> col 0..7, rank '1'..'8' -> row 0..7 (white back rank = row 0)
    let c = (file - b'a') as usize;
    let r = (rank - b'1') as usize;
    (r, c)
}

#[inline]
fn parse_uci_move(uci: &str) -> Option<((usize, usize), (usize, usize), Option<PieceType>)> {
    if uci.len() < 4 { return None; }
    let b = uci.as_bytes();
    let from = algebraic_to_idx(b[0], b[1]);
    let to = algebraic_to_idx(b[2], b[3]);
    let promo = if uci.len() >= 5 {
        match uci.as_bytes()[4] as char {
            'q' | 'Q' => Some(PieceType::Queen),
            'r' | 'R' => Some(PieceType::Rook),
            'b' | 'B' => Some(PieceType::Bishop),
            'n' | 'N' => Some(PieceType::Knight),
            _ => None,
        }
    } else { None };
    Some((from, to, promo))
}

// Pick a move from the opening book for the given state.
// Returns a legal (from,to) if a book entry exists and passes validation.
pub fn book_pick(game_state: &GameState) -> Option<((usize, usize), (usize, usize))> {
    use crate::state::fen::writer::game_state_to_fen_string;
    use crate::search::is_deterministic;
    // Build truncated FEN key (first 4 fields)
    let fen = game_state_to_fen_string(*game_state);
    let key: String = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");

    // Find entry
    let entry = BOOK.iter().find(|(k, _)| k == &key)?.1;
    // Deterministic: pick the highest-weight move; tie-break by lexicographical order.
    let chosen = if is_deterministic() {
        let mut best: Option<(&str, u32)> = None;
        for (uci, w) in entry.iter().copied() {
            best = match best {
                None => Some((uci, w)),
                Some((buci, bw)) => {
                    if w > bw || (w == bw && uci < buci) { Some((uci, w)) } else { Some((buci, bw)) }
                }
            };
        }
        let (uci, _w) = best?;
        entry.iter().find(|(u, _)| *u == uci)?
    } else {
        // Weighted pick
        let total: u32 = entry.iter().map(|(_, w)| *w).sum();
        if total == 0 { return None; }
        let pick = rng().random_range(0..total);
        let mut acc = 0u32;
        entry.iter().find(|(_, w)| {
            acc += *w;
            acc > pick
        })?
    };

    // Parse UCI
    let (from, to, _promo) = parse_uci_move(chosen.0)?;
    // Validate against generator to be safe
    let mut gs_clone = *game_state;
    let legals_pairs: Vec<((usize, usize), (usize, usize))> = find_all_valid_moves(&mut gs_clone)
        .iter()
        .map(|(f, t, _)| (*f, *t))
        .collect();
    if legals_pairs.contains(&(from, to)) {
        Some((from, to))
    } else {
        None
    }
}
