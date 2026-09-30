//! Policy checks use real manifests and source files.

#[test]
fn missing_tool_names_install_guidance() {
    let pins = [
        ("alpha", "1.2", "install alpha 1.2"),
        ("beta", "2.0", "install beta 2.0"),
    ];
    let error = xtask::policy::missing_tools(&pins, |_| false).unwrap_err();
    assert!(error.contains("alpha 1.2: install alpha 1.2"), "{error}");
    assert!(error.contains("beta 2.0: install beta 2.0"), "{error}");
}

#[test]
fn core_only_rejects_heap_and_host_imports() {
    for bad in [
        "extern crate alloc;",
        "use std::vec::Vec;",
        "use alloc::vec::Vec;",
    ] {
        assert!(xtask::policy::core_source(bad).is_err(), "{bad}");
    }
    assert!(
        xtask::policy::core_source("#![no_std]\n/// version\npub const VERSION: &str = \"0.1.0\";")
            .is_ok()
    );
}

#[test]
fn target_specific_dependency_cannot_escape_core_only_policy() {
    let manifest = "[target.'cfg(unix)'.dependencies]\nheap = \"1\"\n";
    assert!(xtask::policy::normal_dependencies(manifest).is_err());
}

#[test]
fn nested_library_module_cannot_import_alloc() {
    let path = std::env::temp_dir().join(format!("helium-core-scan-{}", std::process::id()));
    std::fs::create_dir_all(path.join("nested")).unwrap();
    std::fs::write(path.join("nested/body.rs"), "extern crate alloc;").unwrap();
    let result = xtask::policy::core_tree(&path);
    std::fs::remove_dir_all(&path).unwrap();
    assert!(result.unwrap_err().contains("alloc"));
}

#[test]
fn lint_policy_rejects_weakened_workspace_and_missing_optin() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml")).unwrap();
    assert!(
        xtask::policy::lint_policy(&manifest).is_ok(),
        "{:?}",
        xtask::policy::lint_policy(&manifest)
    );
    assert!(
        xtask::policy::lint_policy(&manifest.replace(
            "pedantic = { level = \"deny\"",
            "pedantic = { level = \"warn\""
        ))
        .is_err()
    );
    assert!(xtask::policy::member_optin("[package]\nname = \"x\"").is_err());
}
