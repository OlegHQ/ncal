# shellcheck shell=bash
# Shared helpers for notion-calendar-cli RE scripts.

default_app() {
  echo "${NOTION_CALENDAR_APP:-/Applications/Notion Calendar.app}"
}

require_app() {
  local app
  app="$(default_app)"
  if [[ ! -d "$app" ]]; then
    echo "error: app bundle not found: $app" >&2
    echo "set NOTION_CALENDAR_APP to the correct .app path" >&2
    exit 1
  fi
  echo "$app"
}

contents_dir() {
  local app="$1"
  echo "$app/Contents"
}

main_executable() {
  local contents="$1"
  local exe
  exe="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$contents/Info.plist" 2>/dev/null || true)"
  if [[ -z "$exe" ]]; then
    echo "error: could not read CFBundleExecutable from Info.plist" >&2
    exit 1
  fi
  echo "$contents/MacOS/$exe"
}

asar_path() {
  local contents="$1"
  echo "$contents/Resources/app.asar"
}
