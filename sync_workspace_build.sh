#!/usr/bin/env bash
# Sync workspace dart tool files to app dir for flutter build
# Usage: ./sync_workspace_build.sh <app-name>
# Example: ./sync_workspace_build.sh mobile-customer

set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"
APP="${1:-mobile-customer}"
FLUTTER="$HOME/sdk/flutter"

echo "Syncing workspace build artifacts for $APP..."

# 1. Ensure workspace deps are resolved
cd "$ROOT"
"$FLUTTER/bin/dart" pub get

# 2. Copy package_config.json with absolute paths
python3 -c "
import json, sys
from pathlib import Path
root = Path('$ROOT/.dart_tool/package_config.json')
with open(root) as f:
    config = json.load(f)
root_dot = Path('$ROOT/.dart_tool')
for pkg in config['packages']:
    uri = pkg['rootUri']
    if not uri.startswith('file://'):
        abs_path = (root_dot / uri).resolve()
        pkg['rootUri'] = abs_path.as_uri()
dest = Path('$ROOT/apps/$APP/.dart_tool/package_config.json')
dest.parent.mkdir(parents=True, exist_ok=True)
with open(dest, 'w') as f:
    json.dump(config, f, indent=2)
print(f'  ✓ package_config.json ({len(config[\"packages\"])} packages)')
"

# 3. Copy flutter plugins files
cp "$ROOT/.flutter-plugins" "$ROOT/apps/$APP/.flutter-plugins"
cp "$ROOT/.flutter-plugins-dependencies" "$ROOT/apps/$APP/.flutter-plugins-dependencies"
echo "  ✓ .flutter-plugins"

echo "Done. Now run: cd apps/$APP && flutter build apk --debug --no-pub"
