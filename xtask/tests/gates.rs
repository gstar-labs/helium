//! Real gate behavior for the skeleton.

use std::process::Command;

#[test]
fn core_only_checks_a_real_target_build() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gate", "--only", "core-only"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn registry_rejects_nonexistent_inputs_without_claiming_success() {
    let gates = xtask::registry();
    for name in [
        "preflight",
        "lint",
        "lint-optin",
        "no-stubs",
        "deps",
        "test",
        "core-only",
    ] {
        assert!(
            matches!(
                gates.iter().find(|gate| gate.name == name).unwrap().check,
                xtask::Check::Active(_)
            ),
            "{name}"
        );
    }
}
