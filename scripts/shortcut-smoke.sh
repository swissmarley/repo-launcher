#!/usr/bin/env bash
# Phase 4 exit criterion (docs/PLAN.md §14): a shortcut cold-start launches a web project
# without the main window. Run after `pnpm tauri build --debug` (with `--bundles app` on macOS)
# and `cargo build -p rl`, on a machine where Repo Launcher isn't running.
set -euo pipefail
cd "$(dirname "$0")/.."

case "$(uname -s)" in
  Darwin) OS=macos ;;
  Linux) OS=linux ;;
  *) OS=windows ;;
esac
EXE_SUFFIX=""; [ "$OS" = windows ] && EXE_SUFFIX=".exe"
RL="target/debug/rl$EXE_SUFFIX"
APP="$PWD/target/debug/repo-launcher$EXE_SUFFIX"
BUNDLE="$PWD/target/debug/bundle/macos/Repo Launcher.app"

ID=$("$RL" add ./fixtures/synthetic/manifest-web | awk '{print $1}')
"$RL" install "$ID" --yes
SHORTCUT=$("$RL" shortcut "$ID" --exe "$APP" | head -1 | cut -d' ' -f3-)
echo "shortcut: $SHORTCUT"

case "$OS" in
  macos) LOGS="$HOME/Library/Logs/app.repolauncher.desktop" ;;
  linux) LOGS="${XDG_DATA_HOME:-$HOME/.local/share}/app.repolauncher.desktop/logs" ;;
  windows) LOGS="$(cygpath -u "$LOCALAPPDATA")/app.repolauncher.desktop/logs" ;;
esac
# Only what the app logs from now on counts.
app_log() { cat "$LOGS"/*.log 2>/dev/null || true; }
BEFORE=$(app_log | wc -l)

# Open the shortcut the way the desktop would.
case "$OS" in
  macos)
    # Registers the repolauncher:// link handler, as installing the app does.
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$BUNDLE"
    open "$SHORTCUT"
    ;;
  linux)
    EXEC=$(sed -n 's/^Exec=//p' "$SHORTCUT")
    # No real browser on the runner: "open in the browser" just succeeds.
    BROWSER=/bin/true xvfb-run -a dbus-run-session -- bash -c "exec $EXEC" >/tmp/rl-app.out 2>&1 &
    ;;
  windows)
    powershell -NoProfile -Command "Start-Process -FilePath '$(cygpath -w "$SHORTCUT")'"
    ;;
esac

URL=""
for _ in $(seq 1 120); do
  STATE=$("$RL" runs "$ID")
  case "$STATE" in "running http"*) URL=${STATE#running }; break ;; esac
  sleep 1
done
[ -n "$URL" ] || { echo "the project never started: $STATE"; cat /tmp/rl-app.out 2>/dev/null || true; exit 1; }
curl -fsS -o /dev/null "$URL"
echo "project answered at $URL"

NEW_LOG=$(app_log | tail -n +$((BEFORE + 1)))
echo "$NEW_LOG"
grep -q "background start" <<<"$NEW_LOG"
if grep -q "showing the main window" <<<"$NEW_LOG"; then
  echo "the main window was shown"; exit 1
fi
echo "ok: started from the shortcut without the main window"

# Clean up: the app (its projects stop with it on Windows; elsewhere stop the server too).
case "$OS" in
  windows) taskkill //IM "repo-launcher.exe" //T //F || true ;;
  *) pkill -f "repo-launcher" || true; pkill -f "http.server" || true ;;
esac
"$RL" unshortcut "$ID"
