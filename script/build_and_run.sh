#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="ProxymanClient"
BUNDLE_ID="local.proxyman.client"
MIN_SYSTEM_VERSION="14.0"
APP_VERSION="${PROXYMAN_APP_VERSION:-${GITHUB_REF_NAME:-0.1.0}}"
APP_VERSION="${APP_VERSION#v}"
BUILD_NUMBER="${PROXYMAN_BUILD_NUMBER:-${GITHUB_RUN_NUMBER:-1}}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SWIFT_PACKAGE="$ROOT_DIR/swiftui"
SIDECAR_MANIFEST="$ROOT_DIR/crates/proxyman-core/Cargo.toml"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_BINARY="$APP_MACOS/$APP_NAME"
SIDECAR_BINARY="$APP_RESOURCES/proxyman-sidecar"
INFO_PLIST="$APP_CONTENTS/Info.plist"
ARCHIVE="$DIST_DIR/$APP_NAME-macos.zip"

stop_running() {
  osascript -e "tell application \"$APP_NAME\" to quit" >/dev/null 2>&1 || true
  pkill -x "$APP_NAME" >/dev/null 2>&1 || true
  pkill -f "$APP_BUNDLE/Contents/Resources/proxyman-sidecar" >/dev/null 2>&1 || true
  pkill -f "proxyman-swiftui-client/command.sock" >/dev/null 2>&1 || true
}

build_app() {
  local configuration="${1:-debug}"
  local cargo_args=()
  local swift_args=()
  local rust_profile_dir="debug"

  if [[ "$configuration" == "release" ]]; then
    cargo_args+=(--release)
    swift_args+=(-c release)
    rust_profile_dir="release"
  elif [[ "$configuration" != "debug" ]]; then
    echo "unknown build configuration: $configuration" >&2
    exit 2
  fi

  cargo build "${cargo_args[@]}" --manifest-path "$SIDECAR_MANIFEST" --bin proxyman-sidecar
  swift build "${swift_args[@]}" --package-path "$SWIFT_PACKAGE"

  local swift_bin_path
  swift_bin_path="$(swift build "${swift_args[@]}" --show-bin-path --package-path "$SWIFT_PACKAGE")"
  local built_app_binary="$swift_bin_path/$APP_NAME"
  local built_sidecar_binary="$ROOT_DIR/crates/proxyman-core/target/$rust_profile_dir/proxyman-sidecar"

  rm -rf "$APP_BUNDLE"
  mkdir -p "$APP_MACOS" "$APP_RESOURCES"
  cp "$built_app_binary" "$APP_BINARY"
  cp "$built_sidecar_binary" "$SIDECAR_BINARY"
  chmod +x "$APP_BINARY" "$SIDECAR_BINARY"

  cat >"$INFO_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleShortVersionString</key>
  <string>$APP_VERSION</string>
  <key>CFBundleVersion</key>
  <string>$BUILD_NUMBER</string>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>$MIN_SYSTEM_VERSION</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST
}

open_app() {
  /usr/bin/open -n "$APP_BUNDLE"
}

codesign_app() {
  local identity="${PROXYMAN_CODESIGN_IDENTITY:-}"
  if [[ -z "$identity" ]]; then
    echo "codesign: skipped, PROXYMAN_CODESIGN_IDENTITY is not set" >&2
    return
  fi

  local sidecar_sign_args=(--force --timestamp --options runtime --sign "$identity")
  local app_sign_args=(--force --timestamp --options runtime --sign "$identity")

  if [[ -n "${PROXYMAN_CODESIGN_ENTITLEMENTS:-}" ]]; then
    app_sign_args+=(--entitlements "$PROXYMAN_CODESIGN_ENTITLEMENTS")
  fi

  /usr/bin/codesign "${sidecar_sign_args[@]}" "$SIDECAR_BINARY"
  /usr/bin/codesign "${app_sign_args[@]}" "$APP_BUNDLE"
  /usr/bin/codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
}

archive_app() {
  local archive="${1:-$ARCHIVE}"
  rm -f "$archive"
  (cd "$DIST_DIR" && /usr/bin/ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$archive")
  echo "$archive"
}

should_notarize() {
  case "${PROXYMAN_NOTARIZE:-auto}" in
    1|true|TRUE|yes|YES)
      return 0
      ;;
    0|false|FALSE|no|NO)
      return 1
      ;;
  esac

  [[ -n "${PROXYMAN_NOTARY_KEYCHAIN_PROFILE:-}" ]] && return 0
  [[ -n "${PROXYMAN_NOTARY_APPLE_ID:-}" && -n "${PROXYMAN_NOTARY_TEAM_ID:-}" && -n "${PROXYMAN_NOTARY_PASSWORD:-}" ]]
}

notarytool_args() {
  if [[ -n "${PROXYMAN_NOTARY_KEYCHAIN_PROFILE:-}" ]]; then
    printf '%s\0%s\0' "--keychain-profile" "$PROXYMAN_NOTARY_KEYCHAIN_PROFILE"
    return
  fi

  if [[ -n "${PROXYMAN_NOTARY_APPLE_ID:-}" && -n "${PROXYMAN_NOTARY_TEAM_ID:-}" && -n "${PROXYMAN_NOTARY_PASSWORD:-}" ]]; then
    printf '%s\0%s\0%s\0%s\0%s\0%s\0' \
      "--apple-id" "$PROXYMAN_NOTARY_APPLE_ID" \
      "--team-id" "$PROXYMAN_NOTARY_TEAM_ID" \
      "--password" "$PROXYMAN_NOTARY_PASSWORD"
    return
  fi

  echo "notarize: missing credentials" >&2
  echo "set PROXYMAN_NOTARY_KEYCHAIN_PROFILE or PROXYMAN_NOTARY_APPLE_ID/PROXYMAN_NOTARY_TEAM_ID/PROXYMAN_NOTARY_PASSWORD" >&2
  exit 2
}

notarize_app() {
  local archive="$1"
  if ! should_notarize; then
    echo "notarize: skipped, no notarization credentials are configured" >&2
    return
  fi

  if [[ -z "${PROXYMAN_CODESIGN_IDENTITY:-}" ]]; then
    echo "notarize: PROXYMAN_CODESIGN_IDENTITY is required before notarization" >&2
    exit 2
  fi

  local args=()
  while IFS= read -r -d '' arg; do
    args+=("$arg")
  done < <(notarytool_args)

  xcrun notarytool submit "$archive" --wait "${args[@]}"
  xcrun stapler staple "$APP_BUNDLE"
  xcrun stapler validate "$APP_BUNDLE"
  archive_app "$archive" >/dev/null
}

package_app() {
  build_app release
  codesign_app
  local archive
  archive="$(archive_app "$ARCHIVE")"
  notarize_app "$archive"
  echo "$archive"
}

case "$MODE" in
  run)
    stop_running
    build_app debug
    open_app
    ;;
  stop)
    stop_running
    ;;
  --debug|debug)
    stop_running
    build_app debug
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    stop_running
    build_app debug
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$APP_NAME\""
    ;;
  --telemetry|telemetry)
    stop_running
    build_app debug
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    stop_running
    build_app debug
    open_app
    sleep 1
    pgrep -x "$APP_NAME" >/dev/null
    ;;
  --package|package)
    package_app
    ;;
  *)
    echo "usage: $0 [run|stop|--debug|--logs|--telemetry|--verify|--package]" >&2
    exit 2
    ;;
esac
