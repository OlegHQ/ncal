#!/usr/bin/env bash
set -euo pipefail

# Extract app.asar into research/artifacts/app-asar (gitignored) for rg/node inspection.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

OUT="${1:-$REPO_ROOT/research/artifacts/app-asar}"

app="$(require_app)"
contents="$(contents_dir "$app")"
asar="$(asar_path "$contents")"

if [[ ! -f "$asar" ]]; then
  echo "error: missing $asar" >&2
  exit 1
fi

mkdir -p "$OUT"
echo "Extracting:"
echo "  $asar"
echo "  -> $OUT"
npx --yes @electron/asar extract "$asar" "$OUT"
echo "Done."
