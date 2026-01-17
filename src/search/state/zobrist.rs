use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

// Zobrist hashing: random keys for [piece_type][color][square] plus side-to-move.
// We generate them from a fixed RNG seed to be deterministic across runs.

static mut Z_PIECE: [[u64; 64]; 12] = [[0; 64]; 12];
static mut Z_SIDE: u64 = 0;
static mut Z_CASTLING: [u64; 16] = [0; 16];
static mut Z_EP: [u64; 8] = [0; 8];
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
        for i in 0..16 {
            Z_CASTLING[i] = rng.next_u64();
        }
        for i in 0..8 {
            Z_EP[i] = rng.next_u64();
        }
    }
}

#[inline]
pub fn zobrist_init_once() {
    INIT_FLAG.call_once(init_tables);
}

#[inline]
pub fn compute_zobrist_full(
    board: &Board,
    to_move: Color,
    castling: &crate::state::castling::CastlingRights,
    ep_target: Option<(usize, usize)>
) -> u64 {
    zobrist_init_once();
    let mut key: u64 = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let idx = piece_index(p.get_type(), p.get_color());
                let sq = r * 8 + c;
                unsafe { key ^= Z_PIECE[idx][sq]; }
            }
        }
    }
    if to_move == Color::White {
        unsafe { key ^= Z_SIDE; }
    }
    
    // Castling
    let mut c_idx = 0;
    if castling.white_kingside() { c_idx |= 1; }
    if castling.white_queenside() { c_idx |= 2; }
    if castling.black_kingside() { c_idx |= 4; }
    if castling.black_queenside() { c_idx |= 8; }
    unsafe { key ^= Z_CASTLING[c_idx]; }

    // En Passant
    if let Some((_r, c)) = ep_target {
        unsafe { key ^= Z_EP[c]; }
    }

    key
}

// ============================================================
// INCREMENTAL ZOBRIST UPDATE HELPERS
// These are available for future optimization to true incremental updates
// ============================================================

/// XOR a piece in/out of the key (toggle)
#[inline]
#[allow(dead_code)]
pub fn zobrist_toggle_piece(key: u64, pt: PieceType, color: Color, sq: (usize, usize)) -> u64 {
    zobrist_init_once();
    let idx = piece_index(pt, color);
    let sq_idx = sq.0 * 8 + sq.1;
    unsafe { key ^ Z_PIECE[idx][sq_idx] }
}

/// XOR the side-to-move bit
#[inline]
#[allow(dead_code)]
pub fn zobrist_toggle_side(key: u64) -> u64 {
    zobrist_init_once();
    unsafe { key ^ Z_SIDE }
}

/// Update castling portion of key (XOR out old, XOR in new)
#[inline]
#[allow(dead_code)]
pub fn zobrist_update_castling(key: u64, old_rights: &crate::state::castling::CastlingRights, new_rights: &crate::state::castling::CastlingRights) -> u64 {
    zobrist_init_once();
    let old_idx = castling_index(old_rights);
    let new_idx = castling_index(new_rights);
    unsafe { key ^ Z_CASTLING[old_idx] ^ Z_CASTLING[new_idx] }
}

/// Update en passant portion of key (XOR out old, XOR in new)
#[inline]
#[allow(dead_code)]
pub fn zobrist_update_ep(key: u64, old_ep: Option<(usize, usize)>, new_ep: Option<(usize, usize)>) -> u64 {
    zobrist_init_once();
    let mut k = key;
    if let Some((_, c)) = old_ep {
        unsafe { k ^= Z_EP[c]; }
    }
    if let Some((_, c)) = new_ep {
        unsafe { k ^= Z_EP[c]; }
    }
    k
}

#[inline]
#[allow(dead_code)]
fn castling_index(rights: &crate::state::castling::CastlingRights) -> usize {
    let mut idx = 0;
    if rights.white_kingside() { idx |= 1; }
    if rights.white_queenside() { idx |= 2; }
    if rights.black_kingside() { idx |= 4; }
    if rights.black_queenside() { idx |= 8; }
    idx
}
