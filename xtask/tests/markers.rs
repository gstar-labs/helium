//! Real source scanning distinguishes literals from unfinished source comments.

#[test]
fn marker_in_string_does_not_count_but_comment_does() {
    let source = r#"fn example() { let text = "// TODO"; }"#;
    assert!(xtask::markers::hits(source, "file.rs").is_empty());
    assert_eq!(
        xtask::markers::hits("fn example() {} // TODO: fix", "file.rs").len(),
        1
    );
}

#[test]
fn handles_nested_comments_quoted_char_and_toml() {
    use xtask::markers::{Lang, blank_literals, hits};
    assert_eq!(hits("/* outer /* inner */ TODO */", "file.rs").len(), 1);
    assert!(hits("let ch = 'x'; let escape = '\\n';", "file.rs").is_empty());
    assert!(hits("name = \"TODO\"\n", "file.toml").is_empty());
    assert!(blank_literals("/- outer /- inner -/ TODO -/", Lang::Lean).contains("TODO"));
}

#[test]
fn scan_observes_repository_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    assert!(!xtask::markers::source_files(root).unwrap().is_empty());
    assert!(xtask::markers::scan(root).unwrap().is_empty());
}
