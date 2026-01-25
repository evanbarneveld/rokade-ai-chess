use chess::search::test_support::RepetitionStack;

#[test]
fn repetition_stack_tracks_counts_and_pop() {
    let mut stack = RepetitionStack::new();
    stack.push(10);
    stack.push(20);
    stack.push(10);

    assert_eq!(stack.count(10), 2);
    assert!(stack.contains(&20));

    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.count(10), 1);
}
