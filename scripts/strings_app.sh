#!/usr/bin/env bash
set -euo pipefail

# Usage: strings_app.sh [min_len]
# Caps lines to avoid huge output; increase STRINGS_HEAD in env if needed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

MIN_LEN="${1:-5}"
HEAD_LINES="${STRINGS_HEAD:-400}"

app="$(require_app)"
contents="$(contents_dir "$app")"
exe_path="$(main_executable "$contents")"

emit_strings() {
  local label="$1"
  local path="$2"
  echo "=== strings: $label ==="
  if [[ ! -f "$path" ]]; then
    echo "(skip: not a file: $path)"
    echo
    return 0
  fi
  strings -n "$MIN_LEN" "$path" | head -n "$HEAD_LINES"
  echo
}

emit_strings "main" "$exe_path"

fw="$contents/Frameworks/Electron Framework.framework/Versions/A/Electron Framework"
emit_strings "Electron Framework" "$fw"
