#!/usr/bin/env bash
set -euo pipefail
# Read a single pinned source shared with the preflight gate.
while read -r name version; do
  cargo install --locked "${name}@${version}"
done < <(python3 - <<'PY'
import tomllib
with open('tools/versions.toml', 'rb') as file:
    for name, pin in tomllib.load(file).items():
        print(name, pin['version'])
PY
)
