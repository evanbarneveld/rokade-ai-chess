# Search Speed Improvements

Potential optimizations to increase search depth, ordered by expected impact.

## High Impact

### 5. Futility Pruning at Higher Depths
Current RFP is depth ≤ 2. Extending to depth 3-4 with scaled margins can prune more safely.

### 6. Multi-Cut Pruning
At high depths, if multiple moves fail high, assume the node will fail high and cut early.

## Quick Wins

- **Lazy SMP** - Simple parallel search where threads share TT but search independently
- **Move ordering tuning** - History/killer/counter weights affect node count significantly
- **TT sizing** - Larger TT (64-128MB) reduces re-searching positions
