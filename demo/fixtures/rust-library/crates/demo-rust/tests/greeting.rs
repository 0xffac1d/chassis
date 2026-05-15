use demo_rust::greeting;

// @claim demo.rust.greeting
#[test]
fn returns_stable_greeting() {
    assert_eq!(greeting("Ada"), "hello, Ada");
}

// @claim demo.rust.empty-name
#[test]
fn defaults_blank_name() {
    assert_eq!(greeting("  "), "hello, friend");
}
