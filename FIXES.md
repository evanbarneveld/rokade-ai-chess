# Chess Engine Fixes - Thread Resolution to Attack Detection




## Attack Detection (square_attacked.rs)

### Fix 5.1: CRITICAL - Implement attack bitboards
**Location**: Lines 11-65
**Issue**: 8x8 = 64 iterations per attack query, no caching
**Action**: Use magic bitboards for sliding pieces, precomputed tables for others
**Priority**: CRITICAL (massive performance impact)

### Fix 5.2: Add early return optimization
**Location**: Lines 11-65
**Issue**: Continues searching after finding first attacker
**Action**: Already returns early (return true), but could optimize loop order
**Priority**: Low

### Fix 5.3: Consider caching attack information
**Location**: Entire function
**Issue**: Recomputed on every call
**Action**: Cache during move generation or use incremental updates
**Priority**: High (performance)

---

## Priority

2. **Fix 5.1** (Attack Bitboards) - CRITICAL performance
4. **Fix 5.3** (Attack caching) - HIGH performance
9. **Fix 4.3** (Pseudo-legal moves) - MEDIUM performance
11. All LOW priority items - refactoring/cleanup
