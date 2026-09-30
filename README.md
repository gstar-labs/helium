# Helium

**Experimental, pre-1.0.** Helium is intended to become a core obligation-concurrency library. It is currently a skeleton, not a concurrency runtime. Do not use it with sensitive systems or data.

## What exists today

- An allocation-free `#![no_std]` library exposing `helium::VERSION`.
- A local repository gate: `cargo xtask gate`.

No operation submission, reservation, events, budgets, or execution API exists yet.

## Build and check

Use the toolchain pinned in `rust-toolchain.toml`:

```sh
cargo build
cargo check --lib --target thumbv7em-none-eabi
cargo xtask gate --only preflight
cargo xtask gate
```

The gate requires the tools listed in `tools/versions.toml`. `cargo xtask gate --list` prints its checks; a skipped check names the future stage that supplies its input.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
