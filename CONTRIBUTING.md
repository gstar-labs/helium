# Contributing to Helium

`cargo xtask gate` is the definition of done. The CI configuration runs it on pull requests and merge groups after publication; this repository currently has no remote protection.

1. Work on a `feat/`, `fix/`, `rfc/`, `docs/`, or `chore/` branch (automated updates use `dependabot/`).
2. Add deterministic integration tests (no sleeps), update the changelog for user-visible changes, and run the complete gate.
3. Once a remote exists, submit a pull request for review and merge through the protected queue.

Hard rules: no unfinished-work markers in source, no unused dependencies, strict Clippy `all` and `pedantic`, a reason on every lint exemption, and integration coverage for functions. The library uses `core` only, without heap allocation. Add a dependency only with its first real use.

After publication, defer work through milestone-assigned issues labeled `deferred`, `decision`, `accepted`, or `finding`; close resolved issues with `Fixes #N`. Before publication, do not cite nonexistent issue numbers.
