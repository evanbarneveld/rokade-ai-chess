use chess::search::context::SearchContext;
use chess::search::integration::time_control::{set_time_budget_ms, time_is_up};
use std::time::Duration;

#[test]
fn time_control_reports_expired_budget() {
    let ctx = SearchContext::new();
    set_time_budget_ms(&ctx, 1);
    std::thread::sleep(Duration::from_millis(5));
    assert!(time_is_up(&ctx));
}

#[test]
fn time_control_zero_budget_expires_immediately() {
    let ctx = SearchContext::new();
    set_time_budget_ms(&ctx, 0);
    assert!(time_is_up(&ctx));
}
