//! Functions never entered by an integration test.
//!
//! First-party code is the root library and repository runner.
//!
//! Records are grouped by source location. A generic function yields one record
//! per instantiation, plus zero-count placeholders, so a location counts as
//! covered when any record for it was entered. Closures are skipped: they are
//! covered through the function that defines them.

use anyhow::{Context as _, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Export {
    data: Vec<Data>,
}

#[derive(Deserialize)]
struct Data {
    functions: Vec<Function>,
}

#[derive(Deserialize)]
struct Function {
    name: String,
    count: u64,
    filenames: Vec<String>,
    regions: Vec<Vec<u64>>,
}

/// A first-party function that no integration test entered.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Untested {
    /// Path relative to the workspace root.
    pub file: String,
    /// First line of the function.
    pub line: u64,
    /// Demangled name.
    pub name: String,
}

/// Library and runner sources. Test
/// files are not first-party code.
fn first_party(relative: &str) -> bool {
    relative.starts_with("xtask/src/") || relative.starts_with("src/")
}

/// Checks an llvm-cov JSON export against the workspace at `root`, skipping the
/// files in `exempt` (relative path → reason).
///
/// # Errors
/// Returns an error naming every untested function and every stale exemption
/// (an exempt file with no function records), or if there are no first-party
/// functions at all (the gate would pass vacuously).
pub fn check(json: &str, root: &str, exempt: &BTreeMap<String, String>) -> anyhow::Result<()> {
    let export: Export = serde_json::from_str(json).context("llvm-cov JSON is not valid")?;
    let prefix = format!("{}/", root.trim_end_matches('/'));
    let mut entered: BTreeMap<(String, u64, u64), (bool, String)> = BTreeMap::new();
    let mut seen_files: BTreeSet<String> = BTreeSet::new();
    for function in export.data.iter().flat_map(|d| &d.functions) {
        let Some(file) = function.filenames.first() else {
            continue;
        };
        let Some(relative) = file.strip_prefix(&prefix) else {
            continue;
        };
        if !first_party(relative) {
            continue;
        }
        seen_files.insert(relative.to_owned());
        let name = format!("{:#}", rustc_demangle::demangle(&function.name));
        if name.ends_with("::<_>")
            || name.ends_with("::{{closure}}")
            || (name.contains("::{closure#") && !name.ends_with("}>"))
            || exempt.contains_key(relative)
        {
            continue;
        }
        let start = function.regions.first();
        let line = start.and_then(|r| r.first()).copied().unwrap_or(0);
        let col = start.and_then(|r| r.get(1)).copied().unwrap_or(0);
        let slot = entered
            .entry((relative.to_owned(), line, col))
            .or_insert((false, name));
        slot.0 |= function.count > 0;
    }
    if seen_files.is_empty() {
        bail!("no first-party functions in the coverage export; the gate would pass vacuously");
    }
    let untested: Vec<Untested> = entered
        .into_iter()
        .filter(|(_, (hit, _))| !hit)
        .map(|((file, line, _), (_, name))| Untested { file, line, name })
        .collect();
    let stale: Vec<&str> = exempt
        .keys()
        .map(String::as_str)
        .filter(|f| !seen_files.contains(*f))
        .collect();
    if !untested.is_empty() || !stale.is_empty() {
        let listed: Vec<String> = untested
            .iter()
            .map(|u| format!("{}:{} {}", u.file, u.line, u.name))
            .collect();
        bail!(
            "functions no integration test enters:\n  {}\nstale exemptions: {stale:?}",
            listed.join("\n  ")
        );
    }
    Ok(())
}
