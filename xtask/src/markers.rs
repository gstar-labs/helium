//! The whole-tree scan for unfinished-work markers.
//!
//! Only comments and code are scanned. The contents of string and character
//! literals are blanked first, so fixtures and patterns never report
//! themselves. Files are listed by git, so ignored and privately excluded
//! paths are never read.

use anyhow::{Context as _, bail};
use regex::{Regex, RegexBuilder};
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

static MARKER: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(concat!(
        r"\b(TODO|FIXME|HACK|XXX)\b|\btodo!\(|\bunimplemented!\(|\bstub\b",
        r"|in an? (full|real|proper|complete|future) implementation",
        r"|\bplaceholder\b|\bfor now\b|\bnot (yet )?implemented\b",
        r"|\btemporary:|\bsimplified:|\bis a follow-up\b",
    ))
    .case_insensitive(true)
    .build()
    .expect("the marker pattern is a valid regex")
});

static ALLOW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"helium:allow-stub \(\s*[^)\s][^)]*\)").expect("the allow pattern is a valid regex")
});

/// Source languages the scan understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// `.rs`
    Rust,
    /// `.toml`
    Toml,
    /// `.lean`
    Lean,
}

impl Lang {
    /// The language of `path`, by extension.
    #[must_use]
    pub fn of(path: &str) -> Option<Self> {
        match Path::new(path).extension().and_then(|e| e.to_str()) {
            Some("rs") => Some(Self::Rust),
            Some("toml") => Some(Self::Toml),
            Some("lean") => Some(Self::Lean),
            _ => None,
        }
    }
}

/// Returns `source` with the contents of every string and character literal
/// replaced by spaces. Line breaks are kept, so line numbers don't change;
/// comments and code are kept as they are.
#[must_use]
pub fn blank_literals(source: &str, lang: Lang) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let rest = &chars[i..];
        i += match lang {
            Lang::Rust => rust_token(rest, &mut out),
            Lang::Toml => toml_token(rest, &mut out),
            Lang::Lean => lean_token(rest, &mut out),
        };
    }
    out
}

fn starts(rest: &[char], text: &str) -> bool {
    let mut it = rest.iter();
    text.chars().all(|c| it.next() == Some(&c))
}

fn blank(c: char) -> char {
    if c == '\n' { '\n' } else { ' ' }
}

fn copy_line(rest: &[char], out: &mut String) -> usize {
    let n = rest
        .iter()
        .position(|&c| c == '\n')
        .map_or(rest.len(), |p| p + 1);
    out.extend(&rest[..n]);
    n
}

fn copy_nested(rest: &[char], out: &mut String, open: &str, close: &str) -> usize {
    let (open_len, close_len) = (open.chars().count(), close.chars().count());
    let mut depth = 0usize;
    let mut i = 0;
    while i < rest.len() {
        if starts(&rest[i..], open) {
            depth += 1;
            out.push_str(open);
            i += open_len;
        } else if starts(&rest[i..], close) {
            out.push_str(close);
            i += close_len;
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return i;
            }
        } else {
            out.push(rest[i]);
            i += 1;
        }
    }
    i
}

/// Keeps `open` and `close`, and blanks everything between them. With
/// `escapes`, a backslash also blanks the character after it.
fn blank_quoted(rest: &[char], out: &mut String, open: &str, close: &str, escapes: bool) -> usize {
    out.push_str(open);
    let mut i = open.chars().count();
    while i < rest.len() {
        if starts(&rest[i..], close) {
            out.push_str(close);
            return i + close.chars().count();
        }
        if escapes && rest[i] == '\\' && i + 1 < rest.len() {
            out.push(' ');
            out.push(blank(rest[i + 1]));
            i += 2;
            continue;
        }
        out.push(blank(rest[i]));
        i += 1;
    }
    i
}

/// `'x'` or an escape such as `'\n'`, `'\''`, `'\u{1F600}'`; `None` for
/// lifetimes, labels and identifiers.
fn char_literal(rest: &[char], out: &mut String) -> Option<usize> {
    if rest.get(1) == Some(&'\\') {
        // The quote closing the longest escape, `'\u{10FFFF}'`, is within these 10 chars.
        let close = rest.iter().skip(3).take(10).position(|&c| c == '\'')? + 3;
        out.push('\'');
        out.extend(std::iter::repeat_n(' ', close - 1));
        out.push('\'');
        return Some(close + 1);
    }
    if rest.get(2) == Some(&'\'') && rest.get(1).is_some_and(|&c| c != '\n' && c != '\'') {
        out.push_str("' '");
        return Some(3);
    }
    None
}

fn rust_token(rest: &[char], out: &mut String) -> usize {
    if starts(rest, "//") {
        return copy_line(rest, out);
    }
    if starts(rest, "/*") {
        return copy_nested(rest, out, "/*", "*/");
    }
    if rest[0] == '"' {
        return blank_quoted(rest, out, "\"", "\"", true);
    }
    if rest[0] == '\''
        && let Some(n) = char_literal(rest, out)
    {
        return n;
    }
    if rest[0].is_alphabetic() || rest[0] == '_' {
        let len = rest
            .iter()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .count();
        let word: String = rest[..len].iter().collect();
        if matches!(word.as_str(), "r" | "br" | "cr") {
            let hashes = rest[len..].iter().take_while(|c| **c == '#').count();
            if rest.get(len + hashes) == Some(&'"') {
                out.push_str(&word);
                let open = format!("{}\"", "#".repeat(hashes));
                let close = format!("\"{}", "#".repeat(hashes));
                return len + blank_quoted(&rest[len..], out, &open, &close, false);
            }
        }
        out.push_str(&word);
        return len;
    }
    out.push(rest[0]);
    1
}

fn toml_token(rest: &[char], out: &mut String) -> usize {
    if rest[0] == '#' {
        return copy_line(rest, out);
    }
    if starts(rest, "\"\"\"") {
        return blank_quoted(rest, out, "\"\"\"", "\"\"\"", true);
    }
    if starts(rest, "'''") {
        return blank_quoted(rest, out, "'''", "'''", false);
    }
    if rest[0] == '"' {
        return blank_quoted(rest, out, "\"", "\"", true);
    }
    if rest[0] == '\'' {
        return blank_quoted(rest, out, "'", "'", false);
    }
    out.push(rest[0]);
    1
}

fn lean_token(rest: &[char], out: &mut String) -> usize {
    if starts(rest, "--") {
        return copy_line(rest, out);
    }
    if starts(rest, "/-") {
        return copy_nested(rest, out, "/-", "-/");
    }
    if rest[0] == '"' {
        return blank_quoted(rest, out, "\"", "\"", true);
    }
    if rest[0] == '\''
        && let Some(n) = char_literal(rest, out)
    {
        return n;
    }
    if rest[0].is_alphanumeric() || rest[0] == '_' {
        let len = rest
            .iter()
            .take_while(|c| c.is_alphanumeric() || matches!(**c, '_' | '\'' | '.'))
            .count();
        out.extend(&rest[..len]);
        return len;
    }
    out.push(rest[0]);
    1
}

/// The lines of `source` (at `path`) that carry an unmarked unfinished-work
/// marker outside literals, as `path:line: text`.
#[must_use]
pub fn offending_lines(path: &str, source: &str, lang: Lang) -> Vec<String> {
    let blanked = blank_literals(source, lang);
    blanked
        .lines()
        .zip(source.lines())
        .enumerate()
        .filter(|(_, (scanned, _))| MARKER.is_match(scanned) && !ALLOW.is_match(scanned))
        .map(|(index, (_, original))| format!("{path}:{}: {}", index + 1, original.trim()))
        .collect()
}

/// Files under `root` that git tracks, or would track: tracked and untracked
/// files, but never files ignored by `.gitignore` or `.git/info/exclude`.
///
/// # Errors
/// Returns an error if git cannot list the files.
pub fn source_files(root: &Path) -> anyhow::Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .context("failed to spawn `git ls-files`")?;
    if !out.status.success() {
        bail!(
            "`git ls-files` failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let listing = String::from_utf8(out.stdout).context("a file name is not UTF-8")?;
    Ok(listing
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Scans every `.rs`, `.toml` and `.lean` file git lists under `root`.
///
/// # Errors
/// Returns an error if listing or reading fails, or if there is nothing to
/// scan (the gate would pass vacuously).
pub fn scan(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut scanned = 0usize;
    let mut hits = Vec::new();
    for name in source_files(root)? {
        let Some(lang) = Lang::of(&name) else {
            continue;
        };
        let path = root.join(&name);
        if !path.is_file() {
            continue;
        }
        let source = std::fs::read_to_string(&path).with_context(|| format!("reading {name}"))?;
        hits.extend(offending_lines(&name, &source, lang));
        scanned += 1;
    }
    if scanned == 0 {
        bail!("no source files to scan; the gate would pass vacuously");
    }
    Ok(hits)
}

/// Report unfinished-work markers in one source file.
#[must_use]
pub fn hits(source: &str, path: &str) -> Vec<String> {
    Lang::of(path).map_or_else(Vec::new, |lang| offending_lines(path, source, lang))
}
