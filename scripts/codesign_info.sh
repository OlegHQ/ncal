#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

app="$(require_app)"

echo "=== codesign display ==="
codesign -dv --verbose=4 "$app" 2>&1 || true
echo

ent="${TMPDIR:-/tmp}/nc-ents.$$"
if codesign -d --entitlements - --xml "$app" >"$ent" 2>/dev/null; then
  echo "=== entitlements (xml) ==="
  if [[ -s "$ent" ]]; then
    cat "$ent"
  else
    echo "(empty)"
  fi
else
  echo "=== entitlements ==="
  echo "(not available or not readable)"
fi
rm -f "$ent"
