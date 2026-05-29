# SwiftUI Client Start Decision

This document freezes the first native-client direction. The goal is to keep the SwiftUI app bound to a stable sidecar contract, not old frontend command names, legacy React event shapes, or a storage implementation.

## 1. IPC

Decision: use Unix domain sockets for the first native boundary.

First shape:

- Rust core runs as a sidecar process owned by the SwiftUI app.
- Command socket: newline-delimited JSON-RPC 2.0 over Unix domain socket.
- Event socket: newline-delimited typed event envelopes over a separate Unix domain socket.
- Socket paths live under the app data/runtime directory and include a per-run token or pid segment to avoid stale collisions.
- SwiftUI talks only to the sidecar command and event sockets.

Initial command frames:

```json
{"jsonrpc":"2.0","id":"1","method":"proxy.availablePort","params":{"apiVersion":1,"host":"127.0.0.1","port":9000}}
```

```json
{"jsonrpc":"2.0","id":"1","method":"proxy.start","params":{"apiVersion":1,"host":"127.0.0.1","port":9000,"findAvailable":false}}
```

For `proxy.availablePort`, `port` is the preferred port. When the proxy is stopped, the sidecar scans upward from that port and returns the first free loopback port. The SwiftUI client calls this during launch/status refresh and displays the returned value before the user starts the proxy.

For `proxy.start`, `port` is the already displayed port selected by the launch/status-refresh preflight. SwiftUI sends `findAvailable: false` so clicking Start does not perform a second port search or mutate the port value after the user has seen it. The sidecar still supports `findAvailable: true` for lower-level socket smoke checks and fallback callers, but the native UI should not rely on that path.

Initial event frame:

```json
{"apiVersion":1,"type":"proxyEvent","payload":{"apiVersion":1,"event":{"kind":"exchangeStarted","payload":{"exchangeId":"...","method":"GET","timestamp":0,"uri":"http://127.0.0.1/"}}}}
```

Keep command request/response traffic separate from the event stream. This makes event backpressure and reconnect behavior easier to test.

## 2. First SwiftUI API Surface

The first client needs only these typed capabilities:

- Proxy lifecycle: start, stop, status.
- Event stream: subscribe to `proxy_event_v1` envelopes.
- Sessions: list in-memory exchange summaries, select an exchange, lazy-load captured request/response body, clear current session.
- Replay: replay a captured request with optional method, URL, header, and body edits.
- HAR: export current in-memory session, import HAR into the current session.
- Rules: load `.rules` text for a pack, validate text, save text, enable/disable pack.
- CA: status, install/trust action result.
- System proxy: enable, disable, status/result text.
- Diagnostics: sidecar health, socket connection state, last error.

Do not expose JSONL rows, legacy `proxy_event`, or old React command shapes.

## 3. macOS System Proxy Test Plan

`curl --proxy http://127.0.0.1:<port>` is valid for proxy-core testing, but plain `/usr/bin/curl` is not a reliable system-proxy test because it does not use macOS System Configuration proxy settings in the way native apps do.

Manual verification should use a native macOS client:

- Start a fake local HTTP proxy on a random loopback port.
- Snapshot active `networksetup` HTTP/HTTPS proxy settings.
- Enable system HTTP/HTTPS proxy to the fake proxy port.
- Verify `networksetup` and `scutil --proxy` report the temporary proxy.
- Run a small Swift `URLSession` request to an HTTP URL and assert the fake proxy receives an absolute-form `GET http://...`.
- Run a Swift `URLSession` request to an HTTPS URL and assert the fake proxy receives `CONNECT host:443`.
- Restore the snapshot and verify the fake proxy no longer receives requests.

CA/MITM verification still needs user-visible Keychain trust approval when macOS prompts for it.

## 4. Rule Engine Deferrals

These rule-engine items are not blockers for the first SwiftUI shell:

- Fault injection execution.
- Body-preview matcher execution.
- Breakpoint-style response actions.
- Rich rule-hit event details beyond current processor-effect metadata.

The first SwiftUI shell can ship with current `.rules` runtime support for redirect, delay, request/response headers, request/response body replacement, map-remote, map-local, and block.

## 5. Session Storage

Decision: the first native shell treats captured traffic as an in-memory session by default.

Expected behavior:

- Starting the app creates a new empty in-memory session.
- Killing or quitting the app drops captured exchanges.
- Explicit HAR export/import is the persistence mechanism for the first shell.
- JSONL remains a transition/debug implementation detail, but SwiftUI should not depend on it.
- SQLite is not needed before the first SwiftUI client unless persistent history, tags, notes, retention policy, or large indexed search become first-shell requirements.

This now uses a `SessionStore` boundary, with an in-memory SwiftUI-facing backend behind the sidecar for summary search, body loading, and session clear.

## First Client Milestone

Build a minimal SwiftUI app that can run before the Rust sidecar socket server exists:

- Shows sidecar connection state.
- Shows proxy start/stop controls.
- Shows a split-view capture list and detail inspector.
- Uses a `CoreClient` protocol with a mock implementation and a Unix-socket implementation.
- Keeps UI layout native and compact, not a port of the React frontend.

Current build/run entrypoint:

```sh
./script/build_and_run.sh
```

This builds `proxyman-sidecar`, builds a local SwiftUI `.app` bundle, embeds the sidecar into the app bundle, opens the client, and lets the client call `proxy.status`, `proxy.availablePort`, `proxy.start`, `proxy.stop`, `sessions.search`, `sessions.loadBody`, `sessions.clear`, `replay.send`, `har.export`, `har.import`, CA, system-proxy, and rule-pack methods over the Unix command socket. Stop it with:

```sh
./script/build_and_run.sh stop
```

Current package entrypoint:

```sh
./script/build_and_run.sh --package
```

This builds release Rust and SwiftPM binaries, creates `dist/ProxymanClient.app`,
embeds `proxyman-sidecar`, writes version/build metadata into `Info.plist`, and
archives `dist/ProxymanClient-macos.zip`. Local packaging is unsigned by default.
Setting `PROXYMAN_CODESIGN_IDENTITY` enables Developer ID signing; setting notary
credentials enables notarization, stapling, and re-archiving.

Current lifecycle behavior:

- SwiftUI defaults to preferred port `9000`.
- On launch/status refresh while stopped, SwiftUI calls `proxy.availablePort` and displays the first free loopback port.
- `proxy.start` sends `findAvailable: false` and starts the already displayed port.
- If another process already listens on `127.0.0.1:9000` or `*:9000`, the launch/status refresh preflight skips to `9001`, then continues upward until it finds a free port.
- The sidecar starts the real proxy core, not sidecar-only mock state.
- The event socket broadcasts sidecar lifecycle frames and `proxyEvent` frames to multiple subscribers.
- SwiftUI consumes `proxyEvent` frames and updates the in-memory capture list from exchange lifecycle, request/response head, body chunk, SSE, WebSocket, and error events.
- The sidecar records typed `proxyEvent` frames into an in-memory session store.
- SwiftUI refreshes session summaries and lazy-loads selected request/response bodies from sidecar body refs.
- SwiftUI `CoreClient` exposes replay, HAR export/import as JSON `Data`, CA, system proxy, and rule-pack operations while keeping arbitrary HAR JSON out of the view model types.

Current stage:

- Minimal lifecycle, live-event capture, session summary refresh, body loading, session clear, replay, HAR import/export, bounded CA/system-proxy/rules socket methods, first SwiftUI rule editing, first CA/system-proxy controls, upstream TLS verification, and read-only system-proxy status are verified by build and socket smoke checks.
- `script/verify-system-proxy-urlsession.swift --service Wi-Fi` passed native `URLSession` system-proxy verification with a fake local proxy and restored the original Wi-Fi proxy settings. This is intentionally a Swift script so the check exercises Foundation `URLSession`, not a separate shell, JS, or `curl` proxy stack.
- The Rules section has a pack list, add/remove, automatic validation/save, matched-size enabled/saved/delete controls beside the selected pack name, and a `.rules` text editor backed by `rules.*` socket methods.
- The Certificates section has CA status/install controls backed by `ca.status` and `ca.install`; the action label switches from `Install` to `Reinstall` once the CA is trusted. It also exposes upstream TLS verification through `upstreamTls.status` and `upstreamTls.update`.
- The System Proxy section has status refresh plus enable/disable controls backed by `systemProxy.status`, `systemProxy.enable`, and `systemProxy.disable`.
- The native layout has a compact source-list sidebar, flat titlebar proxy controls, fixed titlebar control slots to avoid start/stop jitter, and shared `ProxymanButtonStyle` action buttons backed by project-local UI tokens.
- Sidecar CA verification passed on macOS: status was missing before install, install returned true, and status returned installed afterward.
- SwiftPM tests now cover AppModel available-port preflight, Start using the displayed port, event application, selected body loading, capture clear/replay, rule-pack state, CA status/install, upstream TLS verification, system-proxy state, and sidecar JSON-RPC response decoding.
- `Models.swift` now contains value models only. `AppModel`, `CoreClient`, Unix socket sidecar/client code, `ProxyLifecycleStore`, `CaptureSessionsStore`, `RulePacksStore`, `CertificateStore`, and `SystemProxyStore` are split into dedicated flat source files under `swiftui/Sources/ProxymanClient/`.
- Release packaging now has unsigned local fallback plus optional Developer ID signing/notarization wiring. A credentialed notarization pass still requires real CI or local notary credentials.
- Next work keeps the current AppModel store boundaries fixed unless a concrete behavior change creates a new dependency boundary. After release hardening, choose the next post-shell track: persistent session storage or deferred rule-engine expansion.
