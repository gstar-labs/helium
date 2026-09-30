# Changelog

Notable changes to Helium follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, a minor release may break APIs.

## [Unreleased]

Bump: minor

### Added

- A `core`-only, allocation-free reservation and publication model over caller-owned resource, operation, and staging slices.
- Checked canonical slot-plus-generation identities, duplicate-free read/write declarations, and all-or-nothing reservation admission.
- Private staged writes with checked reads and views, prevalidated commit, abort, and explicit terminal-operation recycling.
- Documentation of the model's caller-driven API and its non-goals: capability enforcement, operation execution, threads, events, budgets, fairness, scheduling, and multicore atomicity remain out of scope.
- Repository gate and local development workflow.
