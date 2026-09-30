# Helium

**Experimental, pre-1.0.** Helium is a deterministic, caller-driven reservation and publication model, not a concurrency runtime. Do not use it with sensitive systems or data.

## What exists today

The allocation-free `#![no_std]`, core-only library models whole-object resources with homogeneous `T: Copy` values. All storage belongs to the caller:

- `Resource<T>` stores one committed value and a caller-supplied generation.
- `ResourceId` identifies a resource slot and generation.
- `Operation` and `OperationId` provide caller-owned bounded operation slots and canonical operation identities.
- `Stage<T>` provides caller-owned bounded private staging slots.
- `Model::new(resources, operations, stages)` exclusively borrows those slices for the model's lifetime.

An operation is registered with complete read and write slices, admitted, and then either commits or aborts:

```rust
use helium::{Model, Operation, Resource, ResourceId, Stage};

let mut resources = [Resource::new(0, 11_u32), Resource::new(0, 17_u32)];
let mut operations = [Operation::vacant(), Operation::vacant()];
let mut stages = [Stage::empty(), Stage::empty()];
let mut model = Model::new(&mut resources, &mut operations, &mut stages);
let x = ResourceId::new(0, 0);
let y = ResourceId::new(1, 0);
let empty: [ResourceId; 0] = [];
let writes = [x, y];

let operation = model.register(&empty, &writes)?;
model.admit(operation)?;
model.stage(operation, x, 23)?;
model.stage(operation, y, 29)?;
assert_eq!(model.inspect(x)?, 11); // staged values remain private
model.commit(operation)?;
assert_eq!(model.inspect(x)?, 23);
assert_eq!(model.inspect(y)?, 29);
# Ok::<(), helium::Error>(())
```

`register` validates generations and duplicate-free declarations. A resource may be in both read and write sets, but not twice in either set. `admit` grants the complete footprint or leaves the operation `Ready` without a partial reservation; compatible shared readers and disjoint writers may both be `Running`. `read` accesses a committed value declared for reading. `view` reads an operation's own staged value, including for a write-only resource; without a staged value, a write-only resource does not reveal its old committed value. `stage` replaces an existing candidate in place. `commit` prevalidates and publishes all staged writes, while `abort` discards them and releases the reservation. Terminal operations can be explicitly recycled, which advances their operation generation and makes the old identity stale. Checked errors report stale identities, undeclared access, conflicts, wrong lifecycle states, and bounded-storage capacity failures.

The model's transitions use `&mut self` and therefore have a caller-driven, single-threaded state-machine ordering. A successful commit publishes before returning, and later model calls observe the new committed values. This is not a claim of multicore atomicity.

## Explicit non-goals

This first slice does not execute operation bodies or provide capability checks, trusted-declaration enforcement, threads, event prerequisites, budgets, fairness queues, starvation-freedom guarantees, scheduling policy, or Argon integration. The caller chooses when to register, admit, stage, commit, abort, and recycle. No API here provides thread-safe publication or external authority over resources.

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
