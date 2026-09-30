//! The installer keeps checking pinned packages when a cache is restored.

use std::os::unix::fs::PermissionsExt as _;

#[test]
fn cold_and_warm_runs_install_the_same_pins_into_the_cached_root() {
    let root = std::env::temp_dir().join(format!("helium-ci-installer-{}", std::process::id()));
    let bin = root.join("fake-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CALL_LOG\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let calls = root.join("calls");
    let github_path = root.join("github_path");
    let install_root = root.join("tools");
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    for _ in 0..2 {
        let output = std::process::Command::new("bash")
            .arg("tools/ci-install.sh")
            .current_dir(workspace)
            .env("TOOLS_ROOT", &install_root)
            .env("GITHUB_PATH", &github_path)
            .env("CALL_LOG", &calls)
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let recorded = std::fs::read_to_string(&calls).unwrap();
    let lines: Vec<_> = recorded.lines().collect();
    for (line, package) in lines[..4].iter().zip([
        "cargo-nextest@0.9.146",
        "cargo-shear@1.11.2",
        "cargo-deny@0.20.2",
        "cargo-llvm-cov@0.9.1",
    ]) {
        assert!(line.contains(package), "{line}");
        assert!(line.contains("--locked"), "{line}");
        assert!(
            line.contains(&format!("--root {}", install_root.display())),
            "{line}"
        );
    }
    assert_eq!(lines.len(), 8);
    assert_eq!(&lines[..4], &lines[4..]);
    let path = std::fs::read_to_string(&github_path).unwrap();
    assert_eq!(
        path.lines().collect::<Vec<_>>(),
        vec![install_root.join("bin").to_string_lossy().to_string(); 2]
    );
    std::fs::remove_dir_all(root).unwrap();
}
