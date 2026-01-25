use crate::search::context::SearchContext;

/// Set a hard time budget for the ongoing Search. Passing 0 disables the budget.
pub fn set_time_budget_ms(ctx: &SearchContext, ms: usize) {
    ctx.set_time_budget_ms(ms);
}

/// Check if time budget has expired. Lock-free for performance.
/// Called on every node during search, so must be extremely fast.
#[inline]
pub fn time_is_up(ctx: &SearchContext) -> bool {
    ctx.time_is_up()
}
