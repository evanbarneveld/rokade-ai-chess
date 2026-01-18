# Rokade AI Chess Engine - Architecture Documentation

## Overview

This document explains the internal architecture of the Rokade AI Chess Engine. It focuses on how the engine evaluates positions, searches for moves, and selects the best move to play. Understanding these concepts is essential for maintaining and extending the engine.

---

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Search Architecture](#search-architecture)
3. [Evaluation System](#evaluation-system)
4. [Move Selection](#move-selection)
5. [Code Patterns](#code-patterns)
6. [File Structure](#file-structure)
7. [Development Guide](#development-guide)

---

## Core Concepts

### Score Perspectives

**THE MOST IMPORTANT CONCEPT**: Understanding score perspectives is critical to working with this engine.

#### White-Perspective Scoring (Primary Convention)

The engine uses **White-perspective scoring** as its primary convention:

- **Positive score** = Position favors White
- **Negative score** = Position favors Black
- **Zero score** = Equal/drawn position

**Example**:
- `+300` = White is ahead by 3 pawns
- `-500` = Black is ahead by 5 pawns
- `0` = Material is equal

This is the standard convention used in most chess engines.

#### Side-Relative Scoring (Internal Use)

Some internal heuristics use **side-relative scoring**:

- **Positive score** = Beneficial for the side being evaluated
- **Negative score** = Detrimental for the side being evaluated

These scores must be converted to White-perspective before being used in the main search.

### The Perspective Rule

**Critical Understanding**: After making a move at root level, the search evaluates the resulting position from the **opponent's turn perspective**.

**What this means**:

1. When evaluating White's move (e.g., `e2-e4`):
   - The move is made on the board
   - The resulting position has **Black to move**
   - The search returns a score from **White's perspective**
   - Higher scores favor White (good for White, bad for Black)

2. When evaluating Black's move (e.g., `e7-e5`):
   - The move is made on the board
   - The resulting position has **White to move**
   - The search returns a score from **White's perspective**
   - Lower scores favor Black (bad for White, good for Black)

**Therefore**:
- **White's goal**: MAXIMIZE the score (want higher numbers)
- **Black's goal**: MINIMIZE the score (want lower numbers)

Both colors receive scores in the same (White) perspective, but they optimize in opposite directions!

### Score Conversion: `apply_for_side()`

A utility function that converts side-relative bonuses to White-perspective:

```rust,ignore
pub fn apply_for_side(v: i32, side: Color) -> i32 {
    if side == Color::White { v } else { -v }
}
```

**Usage**:
- `apply_for_side(100, White)` = `+100` (increases White's advantage)
- `apply_for_side(100, Black)` = `-100` (decreases White's advantage = good for Black)

This function ensures heuristics that calculate "good for side" bonuses are correctly converted to the engine's White-perspective scoring system.

---

## Search Architecture

### Overview

The engine uses a **minimax search with alpha-beta pruning** and several optimizations:

```
Root Level
    ↓
Iterative Deepening (depths 1 → target)
    ↓
Alpha-Beta Search (minimax with cutoffs)
    ↓
Quiescence Search (tactical positions)
    ↓
Static Evaluation (leaf nodes)
```

### 1. Root Move Selection (`src/search/core/advanced_search.rs`)

**Entry Point**: `find_best_move_internal()`

**Algorithm**:

```
For each depth from 1 to target_depth:
    For each legal move:
        1. Make the move
        2. Search resulting position with alpha-beta
        3. Unmake the move
        4. Apply heuristic adjustments
        5. Track best move
    Return best move from this depth
```

**Key Features**:

- **Iterative Deepening**: Searches shallow depths first, gradually increasing
  - Provides better move ordering for deeper searches
  - Allows time management (can stop early and use best move so far)
  - Improves transposition table effectiveness

- **Aspiration Windows**: Narrows alpha-beta window around expected score
  - Faster search when guess is correct
  - Re-searches with wider window if guess is wrong

- **Dual Scoring System**:
  - `score_raw`: Pure search evaluation (what the position is worth)
  - `adjusted`: Raw score + heuristic adjustments (tie-breaking hints)

**Example Flow**:

```rust,ignore
// Depth 1: Quick 1-ply search
move e2-e4: raw=-20, adjusted=-10  ← Best so far

// Depth 2: 2-ply search (reuses depth-1 ordering)
move e2-e4: raw=+15, adjusted=+25  ← Best so far
move d2-d4: raw=+10, adjusted=+15

// Depth 3: 3-ply search...
// Returns: e2-e4 (best move from deepest complete iteration)
```

### 2. Alpha-Beta Search (`src/search/core/alphabeta.rs`)

**Core Function**: `alphabeta(game_state, depth, alpha, beta, ply, tt, rep_stack, allow_null_move)`

**Algorithm** (Negamax variant):

```rust,ignore
fn alphabeta(position, depth, alpha, beta) -> score {
    // Terminal conditions
    if depth == 0: return quiescence_search()
    if time_up(): return ABORTED
    if repetition(): return 0  // Draw

    // Transposition table lookup
    if cached_result_available(): return cached_score

    // Null move pruning (try passing the turn)
    if position_not_critical && allow_null_move:
        pass_turn()
        score = -alphabeta(..., depth-R, -beta, -beta+1, ...)
        if score >= beta: return beta  // Cutoff

    // Search all moves
    best_score = -infinity
    for each legal_move:
        make_move()
        score = -alphabeta(..., depth-1, -beta, -alpha, ...)
        unmake_move()

        best_score = max(best_score, score)
        alpha = max(alpha, score)

        if alpha >= beta:
            break  // Beta cutoff (move too good)

    // Store in transposition table
    cache_result(position, best_score, depth)
    return best_score
}
```

**Key Optimizations**:

1. **Transposition Table (TT)**:
   - Caches previously evaluated positions
   - Uses Zobrist hashing for position keys
   - Stores: score, depth, best move, bound type (exact/lower/upper)
   - Special handling: Cached 0 scores near root not trusted (may be stale)

2. **Null Move Pruning**:
   - Tries "passing the turn" to prove position is strong
   - If even after passing we're still winning, we can prune this branch
   - Disabled in critical positions (check, zugzwang risk)

3. **Late Move Reduction (LMR)**:
   - Searches promising moves at full depth
   - Searches unlikely moves at reduced depth first
   - Re-searches at full depth if reduced search surprises

4. **Principal Variation Search (PVS)**:
   - Searches expected best move with full window
   - Searches other moves with null window (scout search)
   - Re-searches with full window if scout finds better move

**Score Returns**:
- Returns score in **White-perspective** (always!)
- Positive = good for White, Negative = good for Black
- Negamax negates scores when switching sides internally

### 3. Quiescence Search (`src/search/core/qsearch.rs`)

**Purpose**: Avoid the "horizon effect" by searching tactical sequences to quiet positions.

**When Used**: At depth=0, instead of immediately evaluating, we search captures and checks.

**Why Needed**:

```
Without Qsearch:
  Depth 4: Evaluate after Qxe5
  → Looks like we won a pawn!

  Reality:
  Depth 4: Qxe5
  Depth 5: Nxe5 (knight recaptures)
  → Actually we traded pieces, not won a pawn

With Qsearch:
  Depth 4: Qxe5 → Continue searching captures
  Depth 5: Nxe5 → Quiet position, now evaluate
  → Correct evaluation: material is equal
```

**Algorithm**:

```rust,ignore
fn qsearch(position, alpha, beta) -> score {
    // Stand pat: can we just stop here?
    stand_pat = evaluate_position()
    if stand_pat >= beta: return beta
    alpha = max(alpha, stand_pat)

    // Try captures and checks only
    for each tactical_move (captures, checks):
        make_move()
        score = -qsearch(..., -beta, -alpha)
        unmake_move()

        if score >= beta: return beta
        alpha = max(alpha, score)

    return alpha
}
```

---

## Evaluation System

### Static Position Evaluation (`src/board/evaluator.rs`)

**Entry Point**: `evaluate_position(board, side_to_move)`

**Returns**: Score in White-perspective (positive = White advantage)

**Components**:

1. **Material Counting**:
   - Pawn = 100, Knight = 300, Bishop = 320, Rook = 500, Queen = 900
   - Adjusted by piece-square tables (position-dependent)

2. **Positional Factors**:
   - King safety (pawn shelter, attack patterns)
   - Piece mobility (number of legal moves)
   - Pawn structure (doubled, isolated, passed pawns)
   - Center control
   - Space advantage

3. **Game Phase**:
   - Opening: Prioritize development, center control
   - Middlegame: Tactical play, king safety
   - Endgame: King activity, passed pawns, pawn races

4. **Special Situations**:
   - Insufficient material → 0 (draw)
   - Opposite-colored bishops → Pull score toward 0
   - Unstoppable passed pawns → Large bonuses

### Root-Level Heuristics (`src/search/management/root_moves.rs`)

At the root level (where the engine chooses its move), additional heuristics provide tie-breaking guidance when moves have similar raw evaluations.

**Function**: `adjust_root_score()`

**Heuristic Pipeline** (applied in order):

1. **Development/Centralization Bonus**
   - Rewards moving pieces to strong squares
   - Encourages knight/bishop development
   - Penalizes early queen moves in opening

2. **SEE (Static Exchange Evaluation) Penalties**
   - Evaluates piece exchanges on destination square
   - Penalizes hanging pieces or bad trades
   - Example: Don't move knight where it can be captured for free

3. **Threat Resolution & Evacuation**
   - Rewards moving threatened pieces to safety
   - Extra bonus for knight evacuation from pawn threats
   - Penalizes ignoring threats

4. **Pawn Attacks on Enemy Pieces**
   - Rewards quiet pawn moves that attack enemy pieces
   - Higher bonus for attacking valuable pieces (knights, rooks, queens)
   - Example: `a6` attacking knight on `b5` gets -150cp bonus

5. **Capture Bonus**
   - Small bonus for capturing (beyond material gain in raw score)
   - Encourages simplification when ahead in material

6. **Endgame Scaling**
   - Adjusts evaluation based on game phase
   - Encourages trading when ahead, avoiding trades when behind
   - 50-move rule awareness

7. **King Safety**
   - Bonus for castling in opening
   - Penalty for exposing king
   - Rewards keeping pawns in front of king

8. **Self-Hanging Detection**
   - Heavy penalty for leaving pieces undefended
   - Bonus for checking moves (forcing opponent response)

9. **Queen Positioning**
   - Bonus for queen pressure on opponent kingside
   - Encourages attacking play

10. **Opponent Tactics**
    - Penalty if move allows opponent knight forks/checks
    - Forward-looking tactical awareness

**Key Implementation Detail**:

All heuristics calculate bonuses in side-relative terms (positive = good for side), then use `apply_for_side()` to convert to White-perspective:

```rust,ignore
let bonus = calculate_bonus();
adjusted += apply_for_side(bonus, side);
```

This ensures Black's bonuses correctly decrease the White-perspective score.

---

## Move Selection

### Root Move Selection Logic (`src/search/evaluation/root_evaluator.rs`)

**Core Function**: `evaluate_root_for_bounds()`

**Selection Process**:

```rust,ignore
// Initialize based on color
let mut best_adjusted = if active_color == Color::White {
    MIN_EVAL_VALUE  // Start from -infinity
} else {
    MAX_EVAL_VALUE  // Start from +infinity
};

// Evaluate each move
for each move in legal_moves:
    adjusted_score = evaluate_and_adjust(move)

    // Color-dependent comparison
    let is_better = if active_color == Color::White {
        adjusted_score > best_adjusted  // White maximizes
    } else {
        adjusted_score < best_adjusted  // Black minimizes
    };

    if is_better {
        best_move = move
        best_adjusted = adjusted_score
    }
```

**Why This Works**:

Since all scores are in White-perspective:
- White wants **higher** scores → maximize
- Black wants **lower** scores → minimize

**Parallel Search** (depth ≥ 6, moves ≥ 4):

1. Search first (most promising) move serially
2. Search remaining moves in parallel using thread pool
3. Merge results using color-appropriate comparison
4. Each parallel task uses local transposition table (no contention)

---

## Code Patterns

### Pattern 1: Heuristic Implementation

When implementing a new heuristic:

```rust,ignore
pub fn my_new_heuristic(
    board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    let mut bonus = 0;

    // Calculate bonus in side-relative terms
    // (positive = good for 'side')
    if some_good_condition {
        bonus += 100;
    }

    // Convert to White-perspective before returning
    apply_for_side(bonus, side)
}
```

**Rule**: Calculate in side-relative terms, convert with `apply_for_side()`.

### Pattern 2: Move Comparison

When comparing moves:

```rust,ignore
let is_better = if color == Color::White {
    new_score > current_best  // White maximizes
} else {
    new_score < current_best  // Black minimizes
};
```

**Rule**: Never use single comparison for both colors with White-perspective scores.

### Pattern 3: Score Interpretation

When debugging or analyzing scores:

```rust,ignore
// White-perspective score
let score = evaluate(position);

if score > 0 {
    println!("White is ahead by {} centipawns", score);
} else if score < 0 {
    println!("Black is ahead by {} centipawns", -score);
} else {
    println!("Position is equal");
}

// For White's move: want score to increase
// For Black's move: want score to decrease
```

### Pattern 4: Negamax Recursion

The negamax framework handles perspective flipping:

```rust,ignore
// Current position from current_side's perspective
let score = alphabeta(...);

// Recurse to opponent's perspective
make_move();
let opponent_score = alphabeta(...);
unmake_move();

// Flip perspective back: opponent's score negated
let score_for_current = -opponent_score;
```

This is handled internally by the engine; heuristics should not negate scores manually.

---

## File Structure

```
src/
├── search/
│   ├── core/
│   │   ├── advanced_search.rs     # Root move search, iterative deepening, move selection
│   │   ├── alphabeta.rs           # Main alpha-beta search algorithm
│   │   └── qsearch.rs             # Quiescence search (tactical extensions)
│   │
│   ├── management/
│   │   ├── root_moves.rs          # Root move evaluation, heuristic adjustments
│   │   ├── move_generator.rs     # Legal move generation
│   │   ├── see.rs                 # Static Exchange Evaluation
│   │   └── prune_null_moves.rs   # Null move pruning logic
│   │
│   ├── evaluation/
│   │   ├── root_evaluator.rs     # Root-level evaluation orchestration
│   │   ├── repetition.rs         # Repetition detection and avoidance
│   │   └── root_heuristics/      # Individual heuristic functions
│   │       ├── threat_resolution.rs
│   │       ├── knight_evacuation.rs
│   │       ├── king_safety.rs
│   │       └── utils.rs
│   │
│   ├── state/
│   │   ├── tt.rs                 # Transposition table
│   │   ├── zobrist.rs            # Zobrist hashing
│   │   └── rep_stack.rs          # Repetition tracking
│   │
│   └── integration/
│       ├── time_control.rs       # Time management
│       ├── uci_feedback.rs       # UCI info output
│       └── threading.rs          # Parallel search
│
├── board/
│   ├── evaluator.rs              # Static position evaluation
│   ├── Board.rs                  # Board representation
│   └── checks/
│       └── square_attacked.rs    # Attack detection
│
├── state/
│   └── game_state.rs             # Game state (board + metadata)
│
└── piece/
    └── pieces.rs                 # Piece types and values
```

---

## Development Guide

### Adding a New Heuristic

**Step 1**: Create heuristic function

```rust,ignore
// In src/search/evaluation/root_heuristics/my_heuristic.rs

use crate::board::Board;
use crate::piece::pieces::Color;
use super::utils::apply_for_side;

pub fn my_new_bonus(
    board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    let mut bonus = 0;

    // Your logic here (calculate in side-relative terms)
    // positive = good for side, negative = bad for side

    apply_for_side(bonus, side)  // Convert to White-perspective
}
```

**Step 2**: Integrate into adjustment pipeline

```rust,ignore
// In src/search/management/root_moves.rs::adjust_root_score()

adjusted += my_new_bonus(base_board, side, from, to);
```

### Testing Your Changes

```bash
# Run all move tests
cargo test --test main -- chess::move_tests

# Run specific test with output
cargo test test_blunder_move_5 -- --nocapture

# Run all tests
cargo test
```

### Debugging Techniques

**Add temporary debug output**:

```rust,ignore
// In root_moves.rs
eprintln!("MOVE: {:?}→{:?} raw={} adj={} delta={}",
          from, to, score_raw, adjusted, adjusted - score_raw);
```

**Use debug ranking**:

```rust,ignore
// In tests
let ranks = debug_rank_root_moves(game_state, history, depth);
for (san, adj, raw) in ranks {
    println!("{}: adj={}, raw={}", san, adj, raw);
}
```

### Common Debugging Questions

**Q: Why is the engine choosing move X over move Y?**

1. Check raw scores: `debug_rank_root_moves()`
2. Compare adjustments: Which heuristics differ?
3. Verify perspective: Are scores White-perspective?
4. Check selection logic: Correct maximize/minimize?

**Q: Why are all raw scores 0?**

1. Check TT: Are cached 0 scores being returned?
2. Check depth: Is search deep enough?
3. Check time: Is search being aborted early?
4. Check position: Is it actually balanced?

**Q: Why do adjustments seem backwards?**

1. Verify `apply_for_side()` is used correctly
2. Check that heuristics calculate in side-relative terms
3. Ensure no manual negations for Black

### Performance Optimization Tips

1. **Avoid allocations in hot paths**
   - Search functions are called millions of times
   - Reuse buffers, use stack allocation

2. **Profile before optimizing**
   ```bash
   cargo build --release
   cargo flamegraph --test main -- --nocapture
   ```

3. **Transposition table sizing**
   - Larger TT = better, but diminishing returns
   - Default size is tuned for balance

4. **Parallel search tuning**
   - Increases with depth (more work per move)
   - Minimum 4 moves needed (overhead not worth it otherwise)

---

## Key Principles

### 1. Score Perspective Consistency

**Always** maintain White-perspective in the main search:
- Raw scores from `alphabeta()`: White-perspective ✓
- Adjusted scores: White-perspective ✓
- Move comparisons: Color-dependent (White max, Black min) ✓

### 2. Heuristic Clarity

Heuristics should:
- Calculate bonuses in clear, side-relative terms
- Use `apply_for_side()` for conversion
- Be well-documented (what they detect, why it matters)

### 3. Testing Discipline

- Every change should pass all existing tests
- Add new tests for new heuristics
- Test both White and Black positions

### 4. Code Clarity

Prefer:
```rust,ignore
let better = if side == Color::White {
    score_a > score_b
} else {
    score_a < score_b
};
```

Over:
```rust,ignore
let better = score_a * side_multiplier > score_b * side_multiplier;
```

Explicit is better than clever when dealing with perspectives.

---

## Conclusion

The Rokade AI Chess Engine is built on a foundation of **perspective-consistent scoring**. Understanding that all scores flow through the system in White-perspective, while the two colors optimize in opposite directions, is the key to working effectively with this codebase.

When adding features or fixing issues:
1. **Identify** which perspective each score is in
2. **Verify** conversions happen at the right boundaries
3. **Test** with both colors to ensure symmetry
4. **Document** any new conventions or special cases

This architecture provides a solid foundation for a strong chess engine while remaining maintainable and extensible.

---

*Last Updated: 2025-01-18*
*Version: 1.0*
