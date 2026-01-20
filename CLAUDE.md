# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Build
cargo build                           # Debug build
cargo build --release                 # Release build (optimized)

# Run
cargo run --release                   # CLI mode
cargo run --release --bin rokade-ai-chess-engine  # Pure UCI mode

# Tests
cargo test                            # All tests
cargo test -- --nocapture             # With output
cargo test test_blunder_move_5        # Single test by name
cargo test --test main -- chess::move_tests  # Move test suite
cargo test --test main -- chess::blunder_tests  # Blunder test suite
```

## Architecture Overview

This is a UCI chess engine (~13K lines) implementing minimax search with alpha-beta pruning.

### Core Scoring Convention

**All scores use White-perspective**: positive favors White, negative favors Black. Both colors see scores the same way but optimize differently:
- White **maximizes** (wants higher scores)
- Black **minimizes** (wants lower scores)

Use `apply_for_side(bonus, side)` to convert side-relative bonuses to White-perspective.

### Search Flow

```
find_best_move() → Iterative Deepening (depth 1→target)
    → Alpha-Beta Search (with PVS, null move pruning, LMR)
        → Quiescence Search (captures/checks at depth 0)
            → Static Evaluation
```

### Key Modules

| Path | Purpose |
|------|---------|
| `src/search/core/advanced_search.rs` | Root move search, iterative deepening |
| `src/search/core/alphabeta.rs` | Main alpha-beta algorithm |
| `src/search/core/qsearch.rs` | Quiescence search |
| `src/search/management/root_moves.rs` | Root heuristic adjustments |
| `src/search/evaluation/root_heuristics/` | Individual heuristic functions |
| `src/search/state/tt.rs` | Transposition table |
| `src/board/evaluator.rs` | Static position evaluation |
| `src/board/evaluators/` | Piece-specific evaluators |

### Dual Scoring System at Root

Moves have two scores:
- `score_raw`: Pure minimax evaluation from search
- `adjusted`: Raw + heuristic adjustments (tie-breaking)

### Adding a New Heuristic

1. Create function in `src/search/evaluation/root_heuristics/`:
```rust
pub fn my_bonus(board: &Board, side: Color, from: (usize, usize), to: (usize, usize)) -> i32 {
    let mut bonus = 0;
    // Calculate in side-relative terms (positive = good for side)
    apply_for_side(bonus, side)  // Convert to White-perspective
}
```

2. Call from `src/search/management/root_moves.rs::adjust_root_score()`

### Move Comparison Pattern

```rust
let is_better = if color == Color::White {
    new_score > current_best  // White maximizes
} else {
    new_score < current_best  // Black minimizes
};
```

## Test Structure

- `tests/chess/blunder_tests.rs` - Regression tests for bad moves
- `tests/chess/move_tests.rs` - Move selection validation
- `tests/chess/perft_*.rs` - Move generation correctness (perft)
- `tests/chess/eval_symmetry.rs` - Evaluation consistency

## Binaries

- `rokade-ai-chess` (chess_main.rs): CLI with interactive commands
- `rokade-ai-chess-engine` (engine_main.rs): Pure UCI protocol

## Features

Parallel search is enabled by default via Rayon. Disable with `--no-default-features`.

# always read `ARCHITECTURE.md` first before you suggest any changes
# Never commit your changes
# Never pull or push this repository
# Do not run the whole test suite
