#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE_TMPDIR="${TMPDIR:-/tmp}"
BASE_TMPDIR="${BASE_TMPDIR%/}"
RUNTIME_DIR="$BASE_TMPDIR/proxyman-swiftui-minimal"
COMMAND_SOCKET="$RUNTIME_DIR/command.sock"
EVENT_SOCKET="$RUNTIME_DIR/event.sock"
SIDECAR_PID_FILE="$RUNTIME_DIR/sidecar.pid"
CLIENT_PID_FILE="$RUNTIME_DIR/client.pid"
SIDECAR_LOG="$RUNTIME_DIR/sidecar.log"
CLIENT_LOG="$RUNTIME_DIR/client.log"
APP_BUNDLE="$RUNTIME_DIR/ProxymanClient.app"

stop_process() {
  local pid_file="$1"
  if [[ -f "$pid_file" ]]; then
    local pid
    pid="$(cat "$pid_file")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
    rm -f "$pid_file"
  fi
}

case "${1:-start}" in
  start)
    mkdir -p "$RUNTIME_DIR"
    osascript -e 'tell application "ProxymanClient" to quit' >/dev/null 2>&1 || true
    stop_process "$CLIENT_PID_FILE"
    stop_process "$SIDECAR_PID_FILE"
    pkill -f "$RUNTIME_DIR/ProxymanClient.app/Contents/Resources/proxyman-sidecar" >/dev/null 2>&1 || true
    pkill -f "proxyman-swiftui-client/command.sock" >/dev/null 2>&1 || true
    rm -rf "$APP_BUNDLE"
    rm -f "$COMMAND_SOCKET" "$EVENT_SOCKET" "$SIDECAR_LOG" "$CLIENT_LOG"

    cargo build --manifest-path "$ROOT_DIR/src-tauri/Cargo.toml" --bin proxyman-sidecar
    swift build --package-path "$ROOT_DIR/swiftui/ProxymanClient"

    SIDECAR_BIN="$ROOT_DIR/src-tauri/target/debug/proxyman-sidecar"
    CLIENT_BIN="$(swift build --show-bin-path --package-path "$ROOT_DIR/swiftui/ProxymanClient")/ProxymanClient"

    mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"
    cp "$CLIENT_BIN" "$APP_BUNDLE/Contents/MacOS/ProxymanClient"
    cp "$SIDECAR_BIN" "$APP_BUNDLE/Contents/Resources/proxyman-sidecar"
    chmod +x "$APP_BUNDLE/Contents/MacOS/ProxymanClient" "$APP_BUNDLE/Contents/Resources/proxyman-sidecar"
    cat > "$APP_BUNDLE/Contents/Info.plist" <<'EOF_PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>ProxymanClient</string>
  <key>CFBundleIdentifier</key>
  <string>local.proxyman.client</string>
  <key>CFBundleName</key>
  <string>ProxymanClient</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF_PLIST

    open -n "$APP_BUNDLE"

    APP_RUNTIME_DIR="$BASE_TMPDIR/proxyman-swiftui-client"
    APP_COMMAND_SOCKET="$APP_RUNTIME_DIR/command.sock"

    cat <<EOF
Started Proxyman SwiftUI minimal test.
runtime: $RUNTIME_DIR
app bundle: $APP_BUNDLE
app command socket: $APP_COMMAND_SOCKET

Stop with:
  $0 stop
EOF
    ;;
  stop)
    stop_process "$CLIENT_PID_FILE"
    stop_process "$SIDECAR_PID_FILE"
    osascript -e 'tell application "ProxymanClient" to quit' >/dev/null 2>&1 || true
    pkill -f "$RUNTIME_DIR/ProxymanClient.app/Contents/Resources/proxyman-sidecar" >/dev/null 2>&1 || true
    pkill -f "proxyman-swiftui-client/command.sock" >/dev/null 2>&1 || true
    rm -f "$COMMAND_SOCKET" "$EVENT_SOCKET"
    echo "Stopped Proxyman SwiftUI minimal test."
    ;;
  *)
    echo "Usage: $0 [start|stop]" >&2
    exit 2
    ;;
esac
