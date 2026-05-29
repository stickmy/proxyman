#!/usr/bin/env bash
set -euo pipefail

: "${PROXYMAN_CODESIGN_CERTIFICATE_BASE64:?missing PROXYMAN_CODESIGN_CERTIFICATE_BASE64}"
: "${PROXYMAN_CODESIGN_CERTIFICATE_PASSWORD:?missing PROXYMAN_CODESIGN_CERTIFICATE_PASSWORD}"

RUNNER_TEMP="${RUNNER_TEMP:-/tmp}"
KEYCHAIN_PASSWORD="${PROXYMAN_CODESIGN_KEYCHAIN_PASSWORD:-$(uuidgen)}"
KEYCHAIN_PATH="$RUNNER_TEMP/proxyman-signing.keychain-db"
CERTIFICATE_PATH="$RUNNER_TEMP/proxyman-codesign.p12"

cleanup() {
  rm -f "$CERTIFICATE_PATH"
}

trap cleanup EXIT

rm -f "$KEYCHAIN_PATH" "$CERTIFICATE_PATH"

if ! printf '%s' "$PROXYMAN_CODESIGN_CERTIFICATE_BASE64" | /usr/bin/base64 --decode >"$CERTIFICATE_PATH" 2>/dev/null; then
  printf '%s' "$PROXYMAN_CODESIGN_CERTIFICATE_BASE64" | /usr/bin/base64 -D >"$CERTIFICATE_PATH"
fi

security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security import "$CERTIFICATE_PATH" \
  -P "$PROXYMAN_CODESIGN_CERTIFICATE_PASSWORD" \
  -A \
  -t cert \
  -f pkcs12 \
  -k "$KEYCHAIN_PATH"
security set-key-partition-list \
  -S apple-tool:,apple:,codesign: \
  -s \
  -k "$KEYCHAIN_PASSWORD" \
  "$KEYCHAIN_PATH"
security list-keychains -d user -s "$KEYCHAIN_PATH" $(security list-keychains -d user | tr -d '"')
security default-keychain -s "$KEYCHAIN_PATH"
security find-identity -v -p codesigning "$KEYCHAIN_PATH"
