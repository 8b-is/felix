// Quick sanity test for the root_weight API.
#[test]
fn test_root_weight() {
    let (p, q) = felix_lang::root_weight();
    assert_eq!((p, q), (7, 1)); // kerek root
}
