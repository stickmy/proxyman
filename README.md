## Proxyman

[![Release](https://github.com/stickmy/proxyman/actions/workflows/release.yml/badge.svg)](https://github.com/stickmy/proxyman/actions/workflows/release.yml)

A native macOS HTTP debugging proxy. The app surface is SwiftUI and the proxy core is Rust.

## Current App Shape

- macOS client: `swiftui`
- Rust sidecar and proxy core: `crates/proxyman-core`
- Run entrypoint: `./script/build_and_run.sh`
- Codex Run action: `.codex/environments/environment.toml`

The old React/Tauri frontend has been removed. The Rust proxy core and sidecar now live in `crates/proxyman-core`; this crate does not depend on Tauri or compile a Tauri app shell.

## Run

```sh
./script/build_and_run.sh
```

Verify build and launch:

```sh
./script/build_and_run.sh --verify
```

Build checks:

```sh
cargo check --manifest-path crates/proxyman-core/Cargo.toml --bin proxyman-sidecar
swift build --package-path swiftui
```

Package a local macOS app archive:

```sh
./script/build_and_run.sh --package
```

By default this creates an unsigned local archive at `dist/ProxymanClient-macos.zip`.

Sign the archive with a Developer ID Application identity:

```sh
PROXYMAN_CODESIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
  ./script/build_and_run.sh --package
```

Sign and notarize with App Store Connect notary credentials:

```sh
PROXYMAN_CODESIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
PROXYMAN_NOTARIZE=1 \
PROXYMAN_NOTARY_APPLE_ID="apple@example.com" \
PROXYMAN_NOTARY_TEAM_ID="TEAMID" \
PROXYMAN_NOTARY_PASSWORD="app-specific-password" \
  ./script/build_and_run.sh --package
```

The release workflow can import a `.p12` signing certificate and notarize the
archive when these GitHub secrets are configured:

- `PROXYMAN_CODESIGN_CERTIFICATE_BASE64`
- `PROXYMAN_CODESIGN_CERTIFICATE_PASSWORD`
- `PROXYMAN_CODESIGN_IDENTITY`
- `PROXYMAN_NOTARIZE`
- `PROXYMAN_NOTARY_APPLE_ID`
- `PROXYMAN_NOTARY_TEAM_ID`
- `PROXYMAN_NOTARY_PASSWORD`
- `PROXYMAN_NOTARY_KEYCHAIN_PROFILE`

Manual macOS system-proxy verification:

```sh
script/verify-system-proxy-urlsession.swift --service Wi-Fi
```

This helper is Swift intentionally. Build/run automation stays in shell, but this check needs to exercise Foundation `URLSession` through the macOS system proxy stack. `curl`, Node, and shell networking can use different proxy resolution paths or override proxy behavior, so they are not equivalent to native app traffic.

## Features

- Support http1, http2, https connections.
- Support request redirect, request delay, response value setting.

## Platform support

Support MacOS(x64, aarch64) only, the Windows is not supported currently.

## chrome with https proxy

If you turned on the global system proxy, when you visit websites based on https, you'll get `NET::ERR_CERT_AUTHORITY_INVALID` error, due to the proxyman using self-signed tls certificates, which chrome does not validate for security reason.

To resolve this issue, follow these steps blow.

- First, click on the website page with your mouse.
- Second, typing the following 12 characters: `thisisunsafe` on keyboard, yeah, there's not any reaction, just like typing a password on a linux terminal.
- Third, press `enter` and the website will be load.

## Rules usages

### Redirect

Each `Redirect` rule is split by a line and divided into two parts by spaces, the first part being the `regex` to be matched and the second part being the final address to be redirected.

**example**

```text
https://proxyhttpbin.org/(.*) https://httpbin.org/$1
```

### Delay

The format is as same as `Redirect` rule, the difference is the second part is the delaying milliseconds.

**example**

```text
# the-uri delaying-milliseconds
https://uri.com 200
```

### Response

The format is as same as `Redirect` rule, the difference is the second part is the `Value File` name. Under the hood, the `Response` rule will find the file which named the second part in value files, then parsing the file content as the response of this request.

```text
https://uri.com uri-response-example
```

#### Value files

The first line is http version and status code. Then the next parts is the response headers until the empty line appear. After the empty line, the parts is response body.

### Troubleshooting

- If you get error messages such as broken dmg files with Apple Silicon machines. Please enter the following command in terminal and restart proxyman.

```sh
sudo xattr -d com.apple.quarantine /Applications/Proxyman.app
```
