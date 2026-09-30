//! Independent repository checks for Helium.

use std::fmt::Write as _;

/// Function-entry coverage checking.
pub mod coverage;
/// Active repository gates.
pub mod gates;
/// Source-marker analysis that ignores string literals.
pub mod markers;
/// Pure policy checks for the repository.
pub mod policy;

/// Whether a gate can run against current inputs.
#[derive(Clone, Copy)]
pub enum Check {
    /// The gate has meaningful inputs.
    Active(fn() -> Result<(), String>),
    /// The gate needs an unavailable future input; this names its milestone.
    Skip(&'static str),
}

/// A registered gate.
pub struct Gate {
    /// Stable command-line name.
    pub name: &'static str,
    /// Implementation or skip reason.
    pub check: Check,
}

impl Gate {
    /// Create an active gate.
    #[must_use]
    pub const fn active(name: &'static str, check: fn() -> Result<(), String>) -> Self {
        Self {
            name,
            check: Check::Active(check),
        }
    }

    /// Create a gate awaiting future inputs.
    #[must_use]
    pub const fn skip(name: &'static str, milestone: &'static str) -> Self {
        Self {
            name,
            check: Check::Skip(milestone),
        }
    }
}

/// All checks, in stable cheap-first order.
#[must_use]
pub fn registry() -> Vec<Gate> {
    const RUNTIME: &str = "M2 · Runtime model";
    const FORMAL: &str = "M3 · Formal model";
    const RELEASE: &str = "M4 · Release";
    vec![
        Gate::active("preflight", gates::preflight),
        Gate::active("fmt", gates::fmt),
        Gate::active("lint", gates::lint),
        Gate::active("lint-optin", gates::lint_optin),
        Gate::active("no-stubs", gates::no_stubs),
        Gate::active("reachability", gates::reachability),
        Gate::active("deps", gates::deps),
        Gate::skip("trusted-base", RUNTIME),
        Gate::skip("duplication", RUNTIME),
        Gate::skip("core-sync", FORMAL),
        Gate::skip("semver", RELEASE),
        Gate::active("test", gates::test),
        Gate::skip("lean", FORMAL),
        Gate::skip("difftest", FORMAL),
        Gate::skip("book", FORMAL),
        Gate::skip("diagnostics", RUNTIME),
        Gate::skip("stability", FORMAL),
        Gate::skip("fuzz", RUNTIME),
        Gate::active("untested-code", gates::untested_code),
        Gate::skip("solver-work", FORMAL),
        Gate::active("core-only", gates::core_only),
    ]
}

/// Run selected checks without hiding later failures.
///
/// # Errors
/// Returns the names and causes of failed checks or an invalid selection.
pub fn run_gates(gates: &[Gate], only: Option<&str>) -> Result<(), String> {
    if let Some(name) = only {
        match gates.iter().find(|gate| gate.name == name) {
            None => return Err(format!("unknown gate `{name}`")),
            Some(Gate {
                check: Check::Skip(reason),
                ..
            }) => {
                return Err(format!("gate `{name}` is not active yet: {reason}"));
            }
            Some(_) => {}
        }
    }
    let mut errors = String::new();
    for gate in gates
        .iter()
        .filter(|gate| only.is_none_or(|name| gate.name == name))
    {
        match gate.check {
            Check::Active(run) => {
                println!("== {} ==", gate.name);
                if let Err(error) = run() {
                    let _ = writeln!(errors, "{}: {error}", gate.name);
                }
            }
            Check::Skip(reason) => println!("SKIP {}: {reason}", gate.name),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
