#!/usr/bin/env bash
set -euo pipefail
# Cargo's install metadata in this root lets a restored cache avoid recompilation.
TOOLS_ROOT="${TOOLS_ROOT:-$HOME/.local/helium-tools}"
mkdir -p "$TOOLS_ROOT/bin"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$TOOLS_ROOT/bin" >> "$GITHUB_PATH"
fi
# Read a single pinned source shared with the preflight gate.
while read -r name version; do
  cargo install --locked --root "$TOOLS_ROOT" "${name}@${version}"
done < <(python3 - <<'PY'
import tomllib
with open('tools/versions.toml', 'rb') as file:
    for name, pin in tomllib.load(file).items():
        print(name, pin['version'])
PY
)
