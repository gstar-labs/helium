//! Commands and repository checks with real input at the skeleton stage.

use crate::policy;
use std::collections::BTreeMap;
use std::process::Command;

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed: {status}", args.join(" ")))
    }
}

fn root() -> Result<std::path::PathBuf, String> {
    Ok(cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| error.to_string())?
        .workspace_root
        .into_std_path_buf())
}

/// Check formatting.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn fmt() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])
}

/// Check all targets against strict Clippy warnings.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn lint() -> Result<(), String> {
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )
}

/// Check presence of every pinned tool and the required target.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn preflight() -> Result<(), String> {
    let pins: toml::Value = toml::from_str(include_str!("../../tools/versions.toml"))
        .map_err(|error| error.to_string())?;
    let entries: Vec<(&str, &str, &str)> = pins
        .as_table()
        .ok_or("empty tool pin file")?
        .iter()
        .map(|(name, entry)| {
            let version = entry
                .get("version")
                .and_then(toml::Value::as_str)
                .unwrap_or("");
            let install = entry
                .get("install")
                .and_then(toml::Value::as_str)
                .unwrap_or("");
            (name.as_str(), version, install)
        })
        .collect();
    if entries.is_empty()
        || entries
            .iter()
            .any(|(_, version, install)| version.is_empty() || install.is_empty())
    {
        return Err("tool pins missing or incomplete".into());
    }
    policy::missing_tools(&entries, |name| {
        let Some(path) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
    })?;
    let targets = Command::new("rustup")
        .args(["target", "list", "--installed", "--toolchain", "1.97.1"])
        .output()
        .map_err(|error| error.to_string())?;
    if !targets.status.success()
        || !String::from_utf8_lossy(&targets.stdout)
            .lines()
            .any(|line| line == "thumbv7em-none-eabi")
    {
        return Err(
            "missing target: rustup target add thumbv7em-none-eabi --toolchain 1.97.1".into(),
        );
    }
    Ok(())
}

/// Check workspace lint settings and every member's opt-in.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn lint_optin() -> Result<(), String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| error.to_string())?;
    let manifest = std::fs::read_to_string(metadata.workspace_root.join("Cargo.toml"))
        .map_err(|error| error.to_string())?;
    policy::lint_policy(&manifest)?;
    for package in metadata.workspace_packages() {
        let manifest =
            std::fs::read_to_string(&package.manifest_path).map_err(|error| error.to_string())?;
        policy::member_optin(&manifest).map_err(|error| format!("{}: {error}", package.name))?;
    }
    Ok(())
}

/// Check the product source and compile it without a host standard library.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn core_only() -> Result<(), String> {
    let base = root()?;
    let manifest =
        std::fs::read_to_string(base.join("Cargo.toml")).map_err(|error| error.to_string())?;
    policy::normal_dependencies(&manifest)?;
    let source =
        std::fs::read_to_string(base.join("src/lib.rs")).map_err(|error| error.to_string())?;
    policy::core_source(&source)?;
    policy::core_tree(&base.join("src"))?;
    let status = Command::new("cargo")
        .args([
            "check",
            "-p",
            "helium",
            "--lib",
            "--target",
            "thumbv7em-none-eabi",
        ])
        .env_remove("RUSTC_WRAPPER")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .env_remove("__CARGO_LLVM_COV_RUSTC_WRAPPER")
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("core-only target build failed: {status}"))
    }
}

/// Check used dependencies and advisories.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn deps() -> Result<(), String> {
    run("cargo", &["shear", "--deny-warnings"])?;
    run("cargo", &["deny", "check"])
}

/// Run integration suites and doctests.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn test() -> Result<(), String> {
    run("cargo", &["nextest", "run", "--workspace"])?;
    run("cargo", &["test", "--workspace", "--doc"])
}

/// Reject actual unfinished-work comments in tracked and untracked sources.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn no_stubs() -> Result<(), String> {
    let hits = crate::markers::scan(&root()?).map_err(|error| error.to_string())?;
    if hits.is_empty() {
        Ok(())
    } else {
        Err(format!("unfinished markers: {}", hits.join("; ")))
    }
}

/// Verify that the library and gate runner are the only workspace members.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn reachability() -> Result<(), String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| error.to_string())?;
    let mut members: Vec<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|pkg| pkg.name.as_str())
        .collect();
    members.sort_unstable();
    if members == ["helium", "xtask"] {
        Ok(())
    } else {
        Err(format!(
            "unexpected or orphan workspace members: {members:?}"
        ))
    }
}

/// Require integration-test entry coverage for every library and gate function.
///
/// # Errors
/// Returns an error if the check cannot complete or its invariant fails.
pub fn untested_code() -> Result<(), String> {
    let base = root()?;
    let output = base.join("target/coverage.json");
    let path = output.to_string_lossy().into_owned();
    run("cargo", &["llvm-cov", "clean", "--workspace"])?;
    run(
        "cargo",
        &[
            "llvm-cov",
            "nextest",
            "--workspace",
            "-E",
            "kind(test)",
            "--json",
            "--output-path",
            &path,
        ],
    )?;
    let data = std::fs::read_to_string(&output).map_err(|error| error.to_string())?;
    // Orchestration wrappers invoke external tools and are exercised by the full gate;
    // running them inside coverage would recursively launch the coverage gate.
    let exempt = BTreeMap::from([
        (
            "xtask/src/gates.rs".to_owned(),
            "gate orchestration".to_owned(),
        ),
        ("xtask/src/main.rs".to_owned(), "CLI entry point".to_owned()),
    ]);
    crate::coverage::check(&data, &base.to_string_lossy(), &exempt)
        .map_err(|error| error.to_string())
}
