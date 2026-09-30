//! Pure repository-policy checks.

/// List every missing executable and the install guidance attached to it.
///
/// # Errors
/// Returns a combined message when one or more executables are missing.
pub fn missing_tools(
    pins: &[(&str, &str, &str)],
    present: impl Fn(&str) -> bool,
) -> Result<(), String> {
    let missing: Vec<String> = pins
        .iter()
        .filter(|(name, _, _)| !present(name))
        .map(|(name, version, install)| format!("{name} {version}: {install}"))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing tools:\n{}", missing.join("\n")))
    }
}

/// Check the product's declared core-only source restrictions.
///
/// # Errors
/// Returns an error when a host or heap facility is imported.
pub fn core_source(source: &str) -> Result<(), String> {
    if !source.contains("#![no_std]") {
        return Err("library must declare #![no_std]".into());
    }
    for forbidden in [
        "extern crate alloc",
        "extern crate std",
        "std::",
        "alloc::",
        "unsafe ",
    ] {
        if source.contains(forbidden) {
            return Err(format!("forbidden library source: {forbidden}"));
        }
    }
    Ok(())
}

/// Recursively check all library Rust modules, including nested modules.
///
/// # Errors
/// Returns the offending path or an I/O error.
pub fn core_tree(path: &std::path::Path) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let child = entry.path();
        if child.is_dir() {
            core_tree(&child)?;
        } else if child.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&child).map_err(|error| error.to_string())?;
            for forbidden in [
                "extern crate alloc",
                "extern crate std",
                "std::",
                "alloc::",
                "unsafe ",
            ] {
                if text.contains(forbidden) {
                    return Err(format!(
                        "{}: forbidden library source: {forbidden}",
                        child.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Reject normal product dependencies, including target-specific ones.
///
/// # Errors
/// Returns an error for a nonempty dependencies table or invalid manifest.
pub fn normal_dependencies(manifest: &str) -> Result<(), String> {
    let value: toml::Value = toml::from_str(manifest).map_err(|error| error.to_string())?;
    let normal = |entry: &toml::Value| {
        entry
            .get("dependencies")
            .and_then(toml::Value::as_table)
            .is_some_and(|deps| !deps.is_empty())
    };
    if normal(&value)
        || value
            .get("target")
            .and_then(toml::Value::as_table)
            .is_some_and(|targets| targets.values().any(normal))
    {
        Err("product library cannot have ordinary dependencies".into())
    } else {
        Ok(())
    }
}

/// Check the workspace lint settings at their actual TOML keys.
///
/// # Errors
/// Returns an error when a required lint is weakened or absent.
pub fn lint_policy(manifest: &str) -> Result<(), String> {
    let value: toml::Value =
        toml::from_str(manifest).map_err(|err| format!("invalid manifest: {err}"))?;
    let lint = value.get("workspace").and_then(|v| v.get("lints"));
    for (group, name, expected) in [
        ("rust", "dead_code", "deny"),
        ("rust", "unreachable_pub", "deny"),
        ("rust", "missing_docs", "deny"),
        ("rust", "unused", "deny"),
        ("rust", "unsafe_code", "forbid"),
        ("clippy", "all", "deny"),
        ("clippy", "pedantic", "deny"),
    ] {
        let setting = lint.and_then(|v| v.get(group)).and_then(|v| v.get(name));
        if setting.and_then(|v| {
            v.as_str()
                .or_else(|| v.get("level").and_then(toml::Value::as_str))
        }) != Some(expected)
        {
            return Err(format!("weakened lint: {group}.{name}"));
        }
    }
    Ok(())
}

/// Verify a workspace member inherits the lint policy.
///
/// # Errors
/// Returns an error when the manifest does not opt in.
pub fn member_optin(manifest: &str) -> Result<(), String> {
    let value: toml::Value =
        toml::from_str(manifest).map_err(|err| format!("invalid manifest: {err}"))?;
    if value
        .get("lints")
        .and_then(|v| v.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err("member lacks [lints] workspace = true".into())
    }
}
