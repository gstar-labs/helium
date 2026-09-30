//! A missed first-party function fails coverage without vacuous success.

use std::collections::BTreeMap;

#[test]
fn missing_function_entry_is_rejected() {
    let input = r#"{"data":[{"functions":[{"name":"uncalled","count":0,"filenames":["/repo/src/lib.rs"],"regions":[[3,1,4,1,0,0,0,0]]}]}]}"#;
    let error = xtask::coverage::check(input, "/repo", &BTreeMap::new()).unwrap_err();
    assert!(error.to_string().contains("src/lib.rs:3"), "{error}");
}

#[test]
fn generic_function_covered_by_one_instantiation_is_not_untested() {
    let input = r#"{"data":[{"functions":[{"name":"xtask::policy::missing_tools::<_>","count":0,"filenames":["/repo/xtask/src/policy.rs"],"regions":[[7,1,9,1]]},{"name":"xtask::policy::missing_tools::<test::{closure#0}>","count":1,"filenames":["/repo/xtask/src/policy.rs"],"regions":[[7,1,9,1]]}]}]}"#;
    xtask::coverage::check(input, "/repo", &BTreeMap::new()).unwrap();
}
