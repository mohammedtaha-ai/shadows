use core_threads::Threads;

#[test]
fn a_placeholder_from_the_harness_is_no_title() {
    assert_eq!(Threads::harness_title("New session"), None);
    assert_eq!(Threads::harness_title("  new   Session. "), None);
    assert_eq!(
        Threads::harness_title("New session handling in Shadows").as_deref(),
        Some("New session handling in Shadows")
    );
}
