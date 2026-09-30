//! Gate-runner behavior visible to an external caller.

use std::sync::atomic::{AtomicUsize, Ordering};
use xtask::{Check, Gate, run_gates};

static RAN: AtomicUsize = AtomicUsize::new(0);
#[allow(
    clippy::unnecessary_wraps,
    reason = "fake gate matches the fallible gate callback signature"
)]
fn ok() -> Result<(), String> {
    RAN.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
fn first() -> Result<(), String> {
    Err("first broke".into())
}
fn second() -> Result<(), String> {
    Err("second broke".into())
}

#[test]
fn reports_all_failures_and_runs_middle_gate() {
    RAN.store(0, Ordering::SeqCst);
    let gates = [
        Gate::active("first", first),
        Gate::active("middle", ok),
        Gate::active("second", second),
    ];
    let error = run_gates(&gates, None).unwrap_err();
    assert!(
        error.contains("first") && error.contains("second"),
        "{error}"
    );
    assert_eq!(RAN.load(Ordering::SeqCst), 1);
}

#[test]
fn refuses_skipped_gate_when_selected() {
    let gates = [Gate::skip("future", "M2 · Runtime model")];
    let error = run_gates(&gates, Some("future")).unwrap_err();
    assert!(error.contains("M2 · Runtime model"), "{error}");
}

#[test]
fn rejects_unknown_gate_before_running_anything() {
    RAN.store(0, Ordering::SeqCst);
    let gates = [Gate::active("known", ok)];
    assert!(
        run_gates(&gates, Some("missing"))
            .unwrap_err()
            .contains("missing")
    );
    assert_eq!(RAN.load(Ordering::SeqCst), 0);
}

#[test]
fn registry_has_unique_names_and_active_format_gate() {
    let gates = xtask::registry();
    let mut names: Vec<_> = gates.iter().map(|gate| gate.name).collect();
    assert_eq!(
        names,
        vec![
            "preflight",
            "fmt",
            "lint",
            "lint-optin",
            "no-stubs",
            "reachability",
            "deps",
            "trusted-base",
            "duplication",
            "core-sync",
            "semver",
            "test",
            "lean",
            "difftest",
            "book",
            "diagnostics",
            "stability",
            "fuzz",
            "untested-code",
            "solver-work",
            "core-only"
        ]
    );
    assert!(
        gates
            .iter()
            .any(|gate| gate.name == "fmt" && matches!(gate.check, Check::Active(_)))
    );
    let skipped: Vec<_> = gates
        .iter()
        .filter_map(|gate| match gate.check {
            Check::Skip(reason) => Some((gate.name, reason)),
            Check::Active(_) => None,
        })
        .collect();
    assert_eq!(
        skipped,
        vec![
            ("trusted-base", "M2 · Runtime model"),
            ("duplication", "M2 · Runtime model"),
            ("core-sync", "M3 · Formal model"),
            ("semver", "M4 · Release"),
            ("lean", "M3 · Formal model"),
            ("difftest", "M3 · Formal model"),
            ("book", "M3 · Formal model"),
            ("diagnostics", "M2 · Runtime model"),
            ("stability", "M3 · Formal model"),
            ("fuzz", "M2 · Runtime model"),
            ("solver-work", "M3 · Formal model")
        ]
    );
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), gates.len());
}
