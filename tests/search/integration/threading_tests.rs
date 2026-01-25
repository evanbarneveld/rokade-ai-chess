use chess::search::test_support::init_rayon_pool_if_needed;

#[test]
fn init_rayon_pool_is_idempotent() {
    init_rayon_pool_if_needed();
    init_rayon_pool_if_needed();
}
