use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

// Zobrist hashing: random keys for [piece_type][color][square] plus side-to-move.
// We generate them from a fixed RNG seed to be deterministic across runs.

static mut Z_PIECE: [[u64; 64]; 12] = [[0; 64]; 12];
static mut Z_SIDE: u64 = 0;
static INIT_FLAG: std::sync::Once = std::sync::Once::new();

#[inline]
fn piece_index(pt: PieceType, c: Color) -> usize {
    let base = match pt {
        PieceType::Pawn => 0,
        PieceType::Knight => 2,
        PieceType::Bishop => 4,
        PieceType::Rook => 6,
        PieceType::Queen => 8,
        PieceType::King => 10,
    };
    base + if c == Color::White { 0 } else { 1 }
}

fn init_tables() {
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    let mut rng = StdRng::seed_from_u64(0x9E3779B97F4A7C15);
    unsafe {
        for i in 0..12 {
            for sq in 0..64 {
                Z_PIECE[i][sq] = rng.next_u64();
            }
        }
        Z_SIDE = rng.next_u64();
    }
}

#[inline]
pub fn zobrist_init_once() {
    INIT_FLAG.call_once(init_tables);
}

#[inline]
pub fn compute_zobrist(board: &Board, to_move: Color) -> u64 {
    zobrist_init_once();
    let mut key: u64 = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let idx = piece_index(p.get_type(), p.get_color());
                let sq = (r * 8 + c) as usize;
                unsafe { key ^= Z_PIECE[idx][sq]; }
            }
        }
    }
    if to_move == Color::White {
        unsafe { key ^= Z_SIDE; }
    }
    key
}
