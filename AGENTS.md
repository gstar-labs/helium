# Agent guide for Helium

Helium is pre-1.0. Prefer a clear design over compatibility shims, subject to semver rules. Treat it as experimental: work only in disposable environments, never on sensitive systems or data.

## Development

- Build: `cargo build`
- All gates: `cargo xtask gate`
- One gate: `cargo xtask gate --only <name>`; list: `cargo xtask gate --list`
- Missing tools: `cargo xtask gate --only preflight` lists installation commands.
- Before ending any turn that changes this repository, run `cargo xtask gate`; if it fails, report the exact blocker and failing command.

The product library is `src/lib.rs`. It is `core`-only and must not allocate. Repository tooling lives under `xtask/`; pins live in `tools/versions.toml`.

Public docs describe current behavior only; development plans and validation narratives do not belong in them. The maintainer creates release tags.
