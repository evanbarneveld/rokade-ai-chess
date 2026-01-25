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

Both colors receive scores in the same (White) perspective, but they optimize in opposite directions.

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

The engine uses a **minimax alpha-beta search** with PVS/LMR, null-move pruning, and a quiescence search. Root search runs iterative deepening with aspiration windows, root ordering, and optional parallel evaluation.

```
Root Level
    -> Opening book (early only)
    -> Iterative Deepening (depth 1 -> target)
    -> Root Evaluation (ordering + heuristics, optional parallel)
    -> Alpha-Beta Search (PVS, LMR, null-move, TT)
    -> Quiescence Search (captures/checks + selective pawn pushes)
    -> Static Evaluation
```

### Detailed Search Flow with Debug Trace Points

The following diagram shows the complete search flow including all major components.
Debug output (when feature 'debug-search' is active) is shown with `[TAG]` prefixes:

```
ITERATIVE DEEPENING (depth 1 -> target)
|
|-- [ID] "depth X of Y, side to move"
|
`-- ASPIRATION WINDOWS (src/search/management/aspiration.rs)
    |
    |-- [ASP] initial bounds [alpha, beta] based on last_score +/- window
    |-- [ASP] attempt #N with alpha, beta
    |   |
    |   |-- FAIL-LOW:  score <= alpha -> widen window downward, retry
    |   |-- FAIL-HIGH: score >= beta  -> widen window upward, retry
    |   `-- SUCCESS:   alpha < score < beta -> accept result
    |
    `-- ROOT EVALUATION (src/search/evaluation/root_evaluator.rs)
        |
        |-- For each root move:
        |   |-- [ROOT] "move XY: raw=R adj=A (+/-delta)"
        |   |-- [ROOT] "NEW BEST ROOT MOVE: XY adj=A"
        |   |
        |   `-- ALPHA-BETA SEARCH (src/search/core/alphabeta.rs)
        |       |
        |       |-- [AB] "ply=P depth=D alpha=A beta=B side=S"
        |       |-- Terminal checks:
        |       |   |-- Time cutoff -> SEARCH_ABORTED
        |       |   |-- Repetition -> 0 (draw)
        |       |   `-- 50-move rule -> 0 (draw)
        |       |
        |       |-- [AB] TT probe (exact/lower/upper)
        |       |-- [AB] Null-move pruning (if allowed)
        |       |
        |       |-- depth == 0?
        |       |   `-- QUIESCENCE SEARCH (src/search/core/qsearch.rs)
        |       |       |-- [QS] "qdepth=D alpha=A beta=B side=S"
        |       |       |-- [QS] "stand_pat=E in_check=C"
        |       |       |-- Stand-pat cutoff (if not in check)
        |       |       |-- Delta pruning (hopeless positions)
        |       |       |-- Captures/promotions only (unless in check)
        |       |       |-- SEE-based filtering (optional)
        |       |       |-- Selective endgame pawn pushes (limited)
        |       |       `-- [QS] "returning best=B"
        |       |
        |       |-- [AB] "searching N moves at ply=P depth=D"
        |       |-- For each move (TT hint, MVV-LVA+SEE, killers, history, counter/continuation):
        |       |   |-- Passed-pawn extension
        |       |   |-- Frontier futility pruning (depth=1)
        |       |   |-- [LMR] "XY: depth D -> D' (reduction=R)"
        |       |   |-- [PVS] "XY: null-window [a, a+1]"
        |       |   `-- [PVS] "RE-SEARCH!" (if scout beats bound)
        |       |
        |       `-- Store result in TT
        |
        `-- Apply root heuristic adjustments (raw -> adjusted)

>>> [ID] "DEPTH D COMPLETE: best=M raw=R adjusted=A"
```

**Debug Output Limits** (to avoid overwhelming output):
- Root evaluation: All root moves logged
- Alpha-beta: Entry logged for ply <= 4, moves/LMR logged for ply <= 3, PVS for ply <= 2
- Quiescence: Logged for qdepth <= 2

**Enable/Disable**: Set feature `debug-search`

### 1. Root Move Selection (`src/search/core/advanced_search.rs`)

**Entry Point**: `find_best_move_internal()`

**Algorithm**:

```
1. Initialize TT and history (clear if deterministic).
2. Generate legal moves; if none, return None.
3. Opening book (early only): return book move if available.
4. Order root moves (checking moves, captures, promotions).
5. Iterative deepening with aspiration windows:
   - evaluate root moves (serial or parallel, depth>=6 and >=4 moves)
   - apply root heuristics and repetition-avoidance bias
   - reorder PV move to front for next iteration
6. Build PV and emit UCI info each iteration.
7. Apply playing-strength selection using cached TT scores + noise.
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

- **Playing Strength Mode**:
  - Uses TT-cached root scores to select suboptimal moves at lower strength
  - Adds Gaussian noise when not deterministic

### 2. Alpha-Beta Search (`src/search/core/alphabeta.rs`)

**Core Function**: `alphabeta(game_state, depth, alpha, beta, ply, tt, rep_stack, allow_null_move)`

**Algorithm** (maximizing/minimizing):

```rust,ignore
fn alphabeta(position, depth, alpha, beta) -> score {
    if time_up(): return SEARCH_ABORTED
    if repetition(): return 0
    if 50_move_rule(): return 0
    if depth == 0: return qsearch()

    // TT probe (exact/lower/upper) updates alpha/beta
    // Null-move pruning if allowed

    moves = ordered_moves(TT_hint, MVV_LVA, killers, history)
    if moves.is_empty(): return mate_or_stalemate_score()

    for each move in moves:
        make_move()
        maybe_extend_passed_pawn()
        score = alphabeta(child, reduced_or_full_depth, alpha, beta)
        unmake_move()

        update best and alpha/beta (maximize for White, minimize for Black)
        if cutoff: break

    store TT entry (bound + best move)
    return best
}
```

**Key Optimizations**:

1. **Transposition Table (TT)**:
   - Lock-free atomic implementation enabling parallel search threads to share cached positions
   - Uses XOR-based corruption detection (stores key^data and data in two AtomicU64 words)
   - 16 bytes per entry with packed data layout for memory efficiency
   - Uses Zobrist hashing for position keys
   - Stores: score, depth, best move, bound type (exact/lower/upper), age
   - Special handling: Cached 0 scores near root not trusted (may be stale)

2. **Null Move Pruning**:
   - Tries "passing the turn" to prove position is strong
   - If even after passing we're still winning, we can prune this branch
   - Uses static-eval gating and dynamic reductions; disabled in critical positions (check, zugzwang risk)

3. **Late Move Reduction (LMR)**:
   - Reduces quiet moves and bad captures after a few moves are searched
   - Logarithmic scaling by depth and move index
   - Reductions are eased for killer/counter/continuation-history moves; checking moves are not reduced
   - Re-searches at full depth if reduced search looks promising

4. **Principal Variation Search (PVS)**:
   - Full window for the first move
   - Null-window scouts for the rest, with re-search on improvement

5. **Other pruning and extensions**:
   - Frontier futility pruning at depth=1 for quiet moves
   - Passed-pawn extension in late endgames

**Score Returns**:
- Returns score in **White-perspective** (always)
- Positive = good for White, Negative = good for Black

### 3. Quiescence Search (`src/search/core/qsearch.rs`)

**Purpose**: Avoid the "horizon effect" by searching tactical sequences to quiet positions.

**When Used**: At depth=0, instead of immediately evaluating, we search captures and checks.

**Algorithm**:

```rust,ignore
fn qsearch(position, alpha, beta) -> score {
    if time_up(): return SEARCH_ABORTED
    if repetition() or 50_move_rule(): return 0

    stand_pat = evaluate_position()
    if not in_check:
        apply stand-pat cutoff
        apply delta pruning

    moves = captures/promotions only (unless in_check)
    optionally filter captures with SEE
    optionally add a few safe passed-pawn pushes in late endgame

    for move in moves (MVV-LVA):
        make_move()
        score = qsearch(child, alpha, beta)
        unmake_move()
        update alpha/beta with cutoffs

    return best
}
```

---

## Evaluation System

### Static Position Evaluation (`src/board/evaluator.rs`)

**Entry Point**: `evaluate_position(board, side_to_move)`

**Returns**: Score in White-perspective (positive = White advantage)

**Components**:

1. **Material Counting**:
   - Pawn = 100, Knight = 320, Bishop = 330, Rook = 500, Queen = 900
   - Tapered piece-square tables are applied by game phase

2. **Positional Factors**:
   - Hanging pieces
   - Piece mobility (pseudo-legal target counts)
   - Holes, center control, and space
   - Pawn structure (islands, chains, tension, storms, majorities)
   - Passed pawn quality (blockades, connected passers, king distance)
   - Rook/queen activity (open/semi-open files, king file, alignment, endgame centralization)
   - King safety, shelter, and endgame activity (open-file pressure, queen scaling)
   - Piece interactions (defended pieces, tropism, batteries)
   - Safe exchange threats against defended pieces
   - Tempo bonus scaled by phase

3. **Special Situations**:
   - Insufficient material -> 0 (draw)
   - Opposite-colored bishops only -> pull score toward zero (0.75x)

### Root-Level Heuristics (`src/search/management/root_moves.rs`)

At the root level (where the engine chooses its move), additional heuristics provide tie-breaking guidance when moves have similar raw evaluations.

**Function**: `adjust_root_score()`

**Heuristic Pipeline** (applied in order):

1. **Development/Centralization Bonus**
   - Light development bias for minors

2. **SEE (Static Exchange Evaluation) Penalties**
   - Penalize bad destination exchanges

3. **Threat Resolution & Evacuation**
   - Reward saving threatened pieces; special handling for pawn threats
   - Quiet pawn moves that attack valuable pieces get a bonus

4. **Knight Evacuation Priority**
   - Extra weight for escaping pawn attacks

5. **Capture Bonus**
   - Small capture bonus based on captured value

6. **Endgame / 50-move Scaling**
   - Scaling for simplification and 50-move pressure

7. **King Safety**
   - Castling and exposure considerations

8. **Self-Hang or Check Mobility**
   - Penalize self-hanging; bonus for safe checks

9. **Queen Kingside Pressure**
   - Bonus for kingside pressure

10. **Opponent Knight Checks/Forks**
    - Penalty if move enables tactical knight threats

11. **Critical Square Defense**
    - f2/f7 defense bonus

**Key Implementation Detail**:

All heuristics calculate bonuses in side-relative terms (positive = good for side), then use `apply_for_side()` to convert to White-perspective:

```rust,ignore
let bonus = calculate_bonus();
adjusted += apply_for_side(bonus, side);
```

This ensures Black's bonuses correctly decrease the White-perspective score.

**Root-Level Post-Processing**:
- Repetition-avoidance bias is applied after adjustment (root only).
- Mate scores are never adjusted; non-mate scores are clamped to avoid flipping losing moves to winning.

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

// Evaluate each move (serial or parallel)
for each move in ordered_root_moves:
    raw_score = evaluate_after_root_move()
    adjusted_score = adjust_root_score() + repetition_bias

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
- White wants **higher** scores -> maximize
- Black wants **lower** scores -> minimize

**Parallel Search** (depth >= 6, moves >= 4):

1. Search first (most promising) move serially to establish PV and bounds
2. Search remaining moves in parallel using Rayon thread pool
3. All threads share a single lock-free transposition table (threads benefit from each other's work)
4. Merge results using color-appropriate comparison

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

### Pattern 4: Maximizing/Minimizing Recursion

The search uses explicit maximize/minimize logic rather than negamax:

```rust,ignore
let maximizing = side_to_move == Color::White;

// Update bounds based on side
if maximizing {
    alpha = alpha.max(score);
} else {
    beta = beta.min(score);
}
```

Heuristics should not negate scores manually.

---

## File Structure

```
src/
+-- search/
¦   +-- core/
¦   ¦   +-- advanced_search.rs     # Root move search, iterative deepening, move selection
¦   ¦   +-- alphabeta.rs           # Main alpha-beta search algorithm
¦   ¦   +-- qsearch.rs             # Quiescence search (tactical extensions)
¦   ¦   +-- simple_search.rs       # Simplified search path
¦   ¦
¦   +-- management/
¦   ¦   +-- aspiration.rs          # Aspiration window logic
¦   ¦   +-- root_moves.rs          # Root move evaluation, heuristic adjustments
¦   ¦   +-- move_generator.rs      # Legal move generation
¦   ¦   +-- see.rs                 # Static Exchange Evaluation
¦   ¦   +-- prune_null_moves.rs    # Null move pruning logic
¦   ¦
¦   +-- evaluation/
¦   ¦   +-- heuristics.rs          # History/killer heuristics
¦   ¦   +-- root_evaluator.rs      # Root-level evaluation orchestration
¦   ¦   +-- repetition.rs          # Repetition detection and avoidance
¦   ¦   +-- root_heuristics/       # Individual heuristic functions
¦   ¦       +-- threat_resolution.rs
¦   ¦       +-- knight_evacuation.rs
¦   ¦       +-- king_safety.rs
¦   ¦       +-- utils.rs
¦   ¦
¦   +-- state/
¦   ¦   +-- tt.rs                  # Lock-free atomic transposition table
¦   ¦   +-- zobrist.rs             # Zobrist hashing
¦   ¦   +-- rep_stack.rs           # Repetition tracking
¦   ¦
¦   +-- integration/
¦       +-- playing_strength.rs    # Strength throttling and noise
¦       +-- telemetry.rs           # Telemetry hooks (currently empty)
¦       +-- time_control.rs        # Time management
¦       +-- uci_feedback.rs        # UCI info output
¦       +-- threading.rs           # Parallel search
¦
+-- board/
¦   +-- evaluator.rs              # Static position evaluation
¦   +-- board.rs                  # Board representation
¦   +-- attack_maps.rs            # Attack maps for evaluation
¦   +-- pst.rs                    # Piece-square tables
¦   +-- san_move.rs               # SAN conversion
¦   +-- evaluators/               # Piece-specific evaluators
¦   +-- checks/
¦       +-- square_attacked.rs    # Attack detection
¦
+-- state/
¦   +-- game_state.rs             # Game state (board + metadata)
¦
+-- piece/
    +-- pieces.rs                 # Piece types and values
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
eprintln!("MOVE: {:?}->{:?} raw={} adj={} delta={}",
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
- Raw scores from `alphabeta()`: White-perspective
- Adjusted scores: White-perspective
- Move comparisons: Color-dependent (White max, Black min)

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

*Last Updated: 2026-01-24*
*Version: 1.2*
