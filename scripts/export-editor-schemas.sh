#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema_dir="$repo_root/editors/schemas"

mkdir -p "$schema_dir"

cargo run --quiet -p lintropy-cli --manifest-path "$repo_root/Cargo.toml" -- \
  schema --kind root --output "$schema_dir/lintropy.schema.json"
cargo run --quiet -p lintropy-cli --manifest-path "$repo_root/Cargo.toml" -- \
  schema --kind rule --output "$schema_dir/lintropy-rule.schema.json"
cargo run --quiet -p lintropy-cli --manifest-path "$repo_root/Cargo.toml" -- \
  schema --kind rules --output "$schema_dir/lintropy-rules.schema.json"
