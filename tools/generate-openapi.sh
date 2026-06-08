#!/usr/bin/env bash
# Generate OpenAPI spec from the Rust Axum router.
# Adds `utoipa` annotations incrementally — start with auth + customer modules,
# then expand. Output: docs/mobile/openapi.json (consumed by mobile codegen).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/docs/mobile/openapi.json"
BIN="$ROOT/target/release/openapi-export"

if [ ! -x "$BIN" ]; then
  echo "Building openapi-export binary..." >&2
  cargo build --manifest-path "$ROOT/src-tauri/Cargo.toml" \
    --bin openapi-export --release >&2
fi

"$BIN" > "$OUT"
echo "✅ OpenAPI spec written to $OUT"
echo "   $(jq '.paths | length' "$OUT") paths, $(jq '.components.schemas | length' "$OUT") schemas"
