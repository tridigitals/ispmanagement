#!/usr/bin/env bash
# Generate Dart models + Dio client from the OpenAPI spec.
# Requires: dart pub global activate openapi_generator_cli
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC="$ROOT/docs/mobile/openapi.json"
OUT="$ROOT/packages/api-client/lib/src/api/generated"

if [ ! -f "$SPEC" ]; then
  echo "❌ OpenAPI spec not found: $SPEC" >&2
  echo "   Run tools/generate-openapi.sh first" >&2
  exit 1
fi

if ! command -v openapi_generator_cli >/dev/null 2>&1; then
  dart pub global activate openapi_generator_cli
fi

openapi_generator_cli generate \
  --input-spec "$SPEC" \
  --generator-name dart-dio \
  --output "$OUT" \
  --additional-properties=supportAsync=true,nullableFields=true

echo "✅ Generated Dart API client at $OUT"
