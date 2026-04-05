#!/usr/bin/env bash
set -euo pipefail

# Lists packaged paths inside app.asar. Requires Node/npx for @electron/asar.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

HEAD_LINES="${ASAR_LIST_HEAD:-800}"

app="$(require_app)"
contents="$(contents_dir "$app")"
asar="$(asar_path "$contents")"

if [[ ! -f "$asar" ]]; then
  echo "error: missing $asar" >&2
  exit 1
fi

echo "=== asar list: $asar ==="
echo "(showing first $HEAD_LINES entries; set ASAR_LIST_HEAD to change)"
echo

npx --yes @electron/asar list "$asar" | head -n "$HEAD_LINES"
