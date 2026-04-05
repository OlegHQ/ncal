#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

app="$(require_app)"
contents="$(contents_dir "$app")"
exe_path="$(main_executable "$contents")"

echo "=== otool -L (main executable) ==="
otool -L "$exe_path"
echo

echo "=== otool -l (LC_RPATH / rpath) ==="
otool -l "$exe_path" | grep -E "path |LC_RPATH" || true
