//! Branch names at the public workflow boundary.

use std::process::Command;

#[test]
fn accepts_only_prefixed_nonempty_branches() {
    for (branch, allowed) in [
        ("feat/feature", true),
        ("dependabot/cargo/update", true),
        ("wip", false),
        ("feat/", false),
    ] {
        let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["branch-name", branch])
            .status()
            .unwrap();
        assert_eq!(status.success(), allowed, "{branch}");
    }
}
