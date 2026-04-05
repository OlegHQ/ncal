#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

app="$(require_app)"
contents="$(contents_dir "$app")"
exe_path="$(main_executable "$contents")"
asar="$(asar_path "$contents")"

echo "=== Notion Calendar bundle snapshot ==="
echo "App: $app"
echo

echo "--- Info.plist (selected) ---"
/usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "$contents/Info.plist" 2>/dev/null | sed 's/^/CFBundleIdentifier: /'
/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$contents/Info.plist" 2>/dev/null | sed 's/^/CFBundleShortVersionString: /'
/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$contents/Info.plist" 2>/dev/null | sed 's/^/CFBundleExecutable: /'
echo

echo "--- Key paths ---"
echo "Main executable: $exe_path"
echo "app.asar: $asar"
if [[ -f "$asar" ]]; then
  echo "app.asar size (bytes): $(stat -f%z "$asar" 2>/dev/null || stat -c%s "$asar" 2>/dev/null)"
else
  echo "app.asar: MISSING"
fi
echo

echo "--- Frameworks (top) ---"
ls "$contents/Frameworks" 2>/dev/null | head -30
echo

echo "--- Resources (non-lproj, sample) ---"
find "$contents/Resources" -maxdepth 1 -type f 2>/dev/null | head -20
