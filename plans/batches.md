# Batch Implementation Plan

The priority is to turn the Rust proxy into a reliable core before building the SwiftUI app. Each batch is intentionally small enough to verify independently.

## Batch 00: Test Harness And Core Boundary

Goal: make proxy behavior testable without the old frontend.

Test cases first:

- Start a local HTTP origin and send a request through the proxy.
- Start a local HTTPS origin through generated certificates or a test MITM fixture.
- Assert request and response lifecycle events are emitted in order.
- Assert proxy start/stop does not depend on a Tauri window.

Implementation scope:

- Introduce a `proxy-core` boundary, either as a new crate or a module that can later be moved.
- Keep Tauri commands as a thin adapter.
- Define a test event sink independent of `app.emit`.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml proxy_core
```

Done when:

- Tests can exercise proxy behavior without React/Tauri UI.
- Current Tauri app still compiles.

Status:

- State: Verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml proxy_core_forwards_http_request_without_tauri -- --nocapture`
- Last verified: 2026-04-28
- Known gaps: The proxy core is still physically inside `src-tauri`; extraction to a standalone crate remains a follow-up once the first behavior tests are in place.

## Batch 01: Correctness Hardening

Goal: remove high-risk crashes and fix basic protocol correctness.

Test cases first:

- Invalid redirect regex returns a validation error instead of panicking.
- Invalid redirect destination URI returns a rule hit error or skips the rule without crashing.
- Unsupported `Content-Encoding` is reported without killing the connection.
- HTTPS CONNECT to `host:custom_port` preserves the custom port.
- Stopping a proxy twice returns a normal error instead of `assert!` panic.

Implementation scope:

- Replace `unwrap`, `expect`, and `panic!` in request path with typed errors.
- Preserve `Authority` host and port when rebuilding tunneled request URIs.
- Return structured processor validation errors.
- Keep behavior changes narrow.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml correctness
```

Done when:

- Malformed rules and malformed upstream responses cannot crash the proxy.
- CONNECT authority behavior is covered by tests.

Status:

- State: Verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml correctness -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: Rule save-time validation is still deferred to Batch 05; Batch 01 currently prevents runtime panics by skipping invalid regex/destinations and returning controlled decode errors.

## Batch 02: Streaming Capture Foundation

Goal: make body forwarding streaming-first.

Test cases first:

- A chunked response reaches the client before the origin finishes sending all chunks.
- A large response above the capture limit is forwarded fully but captured partially.
- A binary body is recorded as metadata plus optional file reference, not lossy UTF-8.
- A slow UI/event sink does not block network forwarding.
- Gzip/br/zstd preview decoding is bounded and does not corrupt forwarded bytes.

Implementation scope:

- Replace full-body `to_bytes` event creation with a body tap.
- Emit `RequestHead`, `RequestBodyChunk`, `RequestFinished`, `ResponseHead`, `ResponseBodyChunk`, `ResponseFinished`, and `ExchangeError` style events.
- Add body capture limits and backpressure policy.
- Store raw body metadata separately from preview text.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml streaming
```

Done when:

- Streaming responses no longer wait for EOF before the client receives data.
- UI capture can be slow or capped without breaking proxy forwarding.

Status:

- State: Verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml streaming -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml core_api_events -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml exchange_error -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: This batch implemented response-side body tap and bounded EOF capture while preserving full forwarding. The typed SwiftUI-facing stream now includes lifecycle completion events, `RequestBodyChunk` from captured request bodies, runtime `ResponseBodyChunk` events, request/response decode `ExchangeError`, body chunk and WebSocket payload `previewEncoding`/`lossyPreview` metadata, and explicit non-blocking `DropNewest` typed-event backpressure. Runtime response chunks and exchange errors are intentionally typed-only and are not written to the legacy JSONL session log. Remaining work before freezing this area: a process/IPC event-stream framing decision.

## Batch 03: SSE

Goal: support server-sent events as a first-class streaming protocol.

Test cases first:

- `Content-Type: text/event-stream` is detected.
- `data:`, `event:`, `id:`, `retry:`, and comment heartbeat lines parse correctly.
- Events split across multiple chunks are reassembled.
- Multi-line `data:` fields are joined according to SSE rules.
- The client connection remains open while parsed SSE events are emitted.
- Malformed lines are preserved as raw stream data without terminating the response.

Implementation scope:

- Add incremental SSE parser.
- Emit structured `SseEvent` messages linked to the parent exchange.
- Keep raw body streaming from Batch 02 intact.
- Add future extension point for SSE rule actions, but do not implement rule injection in this batch.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml sse
```

Done when:

- A long-lived SSE endpoint works through the proxy and the UI/core receives event-level data.

Status:

- State: Verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml sse -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: SSE rule actions such as event injection, filtering, and delay are intentionally deferred to the rule-engine batch. Current support captures and emits parsed SSE events while preserving raw stream forwarding.

## Batch 04: WebSocket

Goal: proxy WebSocket traffic and capture message-level events.

Test cases first:

- HTTP upgrade completes through the proxy.
- Text, binary, ping, pong, and close frames are forwarded both directions.
- Message events record direction, opcode, payload preview, byte length, and timestamp.
- Large binary messages respect capture limits.
- Abrupt client and server closes produce lifecycle errors without panics.

Implementation scope:

- Implement the currently empty WebSocket handler.
- Add bidirectional forwarding.
- Add message capture with bounded preview.
- Keep mutation/breakpoint support out of this batch unless tests require it.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml websocket
```

Done when:

- A local echo server works through the proxy and messages are captured.

Status:

- State: Verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml websocket -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: Text, binary, ping, pong, and close frames now have dedicated forwarding/capture tests. Typed WebSocket messages include stable payload preview metadata (`previewEncoding`, `lossyPreview`, `previewTruncated`) and close frame code/reason. Remaining WebSocket API work before the native UI freeze is process/IPC event-stream framing and any future mutation/breakpoint contract.

## Batch 05: Rule Engine

Goal: replace internal ad hoc parsing with validated typed rules while keeping user-authored rule files easy to edit.

Test cases first:

- Rules validate at save/load time.
- Matchers support method, scheme, host, port, path, query, headers, status, content type, and body preview.
- Actions support redirect, delay, map local, map remote, block, request header/body mutation, response header/body mutation, and fault injection.
- Rule priority and pack enable state are deterministic.
- Rule hit effects are emitted with stable IDs.

Implementation scope:

- Compile regex once.
- Use typed rule structs and versioned serialization.
- Use the action-first `.rules` line format as the user-authored configuration format; do not preserve legacy config file compatibility.
- Split request and response rule phases.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml rules
```

Done when:

- Existing redirect/delay/mock response behavior still works.
- New typed request/response mutation is covered by tests.

Status:

- State: Partially verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml core_api_rules_ -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml typed_rules_runtime -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml core_api_rules_parses_and_formats_map_and_body_line_dsl -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml rules_typed_block_returns_response_without_upstream -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml rules_typed_response_header_mutates_matching_response -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml rules_invalid -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml rules_delay_does_not_block_unmatched_concurrent_request -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml request_header -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml rules_response_header -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: Existing redirect/delay/response rules now compile regex at build/update time, reject invalid regex, process requests from cloned rule snapshots so delayed rules do not block unrelated traffic, and include request/response header mutation via `RequestHeader` and `ResponseHeader`. `core_api::rules` now provides versioned transition DTOs and stable `RuleKind` wrappers over current processor modes and pack operations. Typed rule DTOs plus `validate_typed_rules_v1` now cover rule IDs, priorities, request/response phases, method/scheme/host/port/path/query/header/status/content-type/body-preview matchers, redirect/delay/map/block/header/body/fault actions, regex validation, phase validation, duplicate-id rejection, and deterministic priority/id evaluation order. The `.rules` parser/formatter/persistence path covers redirect, delay, request-header, response-header, block, map-remote, map-local, request-body, and response-body lines, preserves user-authored text, and removes old per-processor files when saving a `.rules` pack. `TypedRuleProcessor` now loads `rules.rules` and executes compatible actions in order before legacy processors in the same pack, including request-side redirect, delay, request-header, request-body, map-remote, map-local, and block, plus response-side response-header, response-body, delay, and typed response block with request URL and status matching. Deferred until after first SwiftUI shell: fault injection execution, body-preview matcher execution, richer breakpoint-style response actions, and rule-hit event details beyond the current processor-effect metadata.

## Batch 06: CA And macOS System Proxy

Goal: make trust and proxy state production-safe on macOS.

Test cases first:

- Per-user root CA generation creates unique private keys.
- Generated leaf certificates contain SAN entries for DNS names and IP addresses.
- Trust status reports installed, missing, and stale CA states.
- System proxy changes preserve and restore prior user settings.
- Multiple network services are detected instead of assuming `Wi-Fi`.

Implementation scope:

- Stop shipping a shared private CA key.
- Store private material in an app-specific secure location, preferably Keychain-backed.
- Add proxy state snapshots and crash recovery.
- Support bypass list and service selection.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml ca system_proxy
```

Manual verification is also required because macOS trust and network settings need real OS state.

Done when:

- The app can enable and disable system proxy without losing the user's original settings.
- CA trust is user-specific and recoverable.

Status:

- State: Partially verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml system_proxy_ -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml ca_ -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: System proxy service discovery no longer hardcodes `Wi-Fi`; enabled network services are parsed from `networksetup -listallnetworkservices`, disabled services are skipped, and proxy settings are applied across detected services. Before enabling Proxyman, existing HTTP/HTTPS proxy states and bypass domains are snapshotted to app data; disable restores that snapshot and uses `Empty` to clear previously empty bypass lists. Root CA material is now generated per local app data path and reused from disk, leaf certificates write DNS SAN or IP SAN according to the requested authority, and the old shared CA certificate/key are no longer bundled. Native `URLSession` system-proxy verification and sidecar CA install/status passed on macOS. Remaining work: explicit service selection, authenticated proxy/PAC/SOCKS preservation, and trust-state reporting beyond the current `security verify-cert` boolean.

## Batch 07: Storage, Replay, And Export

Goal: support real debugging sessions, not just live in-memory viewing.

Test cases first:

- Exchanges are persisted with headers, metadata, timing, and body references.
- Large bodies are lazy-loaded from disk.
- Search filters method, host, path, status, headers, body text, and tags.
- Replay sends an edited request and links the replay to the original exchange.
- HAR export/import round-trips supported fields.

Implementation scope:

- Add SQLite or another durable session store.
- Store large bodies separately with retention policy.
- Add replay request builder.
- Add HAR import/export.

Verification:

```sh
cargo test --manifest-path src-tauri/Cargo.toml storage replay har
```

Done when:

- A captured session can be saved, searched, reopened, replayed, and exported.

Status:

- State: Partially verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml core_api_session_ -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml session_store -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml session_store_search -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml session_store_exports -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml har_timings -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml replay -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml spills -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml imports_har -- --nocapture`; full suite `cargo test --manifest-path src-tauri/Cargo.toml`
- Last verified: 2026-04-28
- Known gaps: Proxy events are now appended to a local JSONL session log under app data for the transition surface, with parent directories created automatically and write failures logged without blocking proxy traffic. Large body strings spill to sidecar body files; JSONL keeps a bounded preview plus `bodyRef`, and the full body can be lazy-loaded through a command/API boundary. The log can be read back and filtered by method, host substring, and response status. Request/response pairs with the same event ID can be exported as HAR 1.2 entries with method, URL, query string, headers, status, status text, body text, started time, total time, and wait timing. HAR 1.2 entries can also be imported into the session log as request/response event pairs. Replay can rebuild a captured request by exchange ID, apply method/URI/header/body edits, and send it to the upstream endpoint through a command/API boundary. Session search/body/HAR/replay now go through a typed `core_api::session::SessionStore` contract; JSONL is only transition support, and a fake non-JSONL store test verifies the facade does not consume raw event values. The first SwiftUI sidecar now has an in-memory `SessionStore` that records typed `proxyEvent` frames and exposes session summary search, body loading, clear, replay, and HAR import/export commands. Typed search supports method, host, path, status, body text, header name, and header value without returning body/header details in list summaries. Decision: do not switch to SQLite before first shell; implement SQLite later only if [session-storage-backend.md](session-storage-backend.md) cutover criteria become first-shell requirements. Remaining work before full native session parity: add HTTPS replay parity and expand HAR phases when needed.

## Batch 08: SwiftUI Shell

Goal: replace the old frontend with a native macOS UI.

Test cases first:

- SwiftUI can start/stop the proxy through the stable core API.
- Event stream updates the connection list without blocking proxy traffic.
- Selecting an exchange lazy-loads body data.
- Rule editing validates before saving.
- System proxy and CA flows report actionable state.

Implementation scope:

- Build a new SwiftUI app and discard the React layout.
- Prefer a small IPC boundary to the Rust core at first, so the proxy can crash/restart independently.
- Use native macOS tables, split views, inspectors, menus, and settings windows.

Verification:

```sh
swift build --package-path swiftui/ProxymanClient
cargo test --manifest-path src-tauri/Cargo.toml
```

Done when:

- The old frontend is no longer needed for daily proxy debugging workflows.

Status:

- State: In progress; lifecycle, live-event, session summary/body, replay, HAR, bounded CA/system-proxy/rules shell, first rule editing UI, first CA/system-proxy controls, CA install/status, native system-proxy effect, and read-only system-proxy status verified
- Test command: `cargo test --manifest-path src-tauri/Cargo.toml core_api_session_ -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml sidecar_ -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml system_proxy -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar`; `cargo build --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar`; `swift build --package-path swiftui/ProxymanClient`; `git diff --check`; `scripts/run-swiftui-minimal-test.sh start`; manual Unix-socket `proxy.availablePort`, lower-level `proxy.start` with `findAvailable: true`, external event-socket `nc -U` subscriber, `curl --proxy`, `scripts/verify-system-proxy-urlsession.swift --service Wi-Fi`, sidecar `ca.status`/`ca.install`, and sidecar `systemProxy.status`
- Last verified: 2026-04-29
- Progress: SwiftUI can start with a bundled sidecar and call lifecycle JSON-RPC over Unix command socket. Lifecycle commands now drive the real proxy core. On launch/status refresh while stopped, SwiftUI calls `proxy.availablePort` so the port field reflects the first free loopback port before Start; the SwiftUI Start action then sends `findAvailable: false` so the displayed port is stable. The lower-level sidecar `proxy.start` path still accepts `findAvailable: true`, starts scanning at preferred port `9000`, skipped an occupied `*:9000`, bound `127.0.0.1:9001`, and successfully forwarded a local HTTP request through `curl --proxy`. The sidecar now wraps core typed events as event-socket `proxyEvent` frames, supports multiple event subscribers, records typed `proxyEvent` frames into an in-memory session store, and exposes `sessions.search`, `sessions.loadBody`, `sessions.clear`, `replay.send`, `har.export`, `har.import`, `ca.status`, `ca.install`, `systemProxy.status`, `systemProxy.enable`, `systemProxy.disable`, `rules.getPackRules`, `rules.validate`, `rules.savePackRules`, `rules.addPack`, `rules.removePack`, and `rules.updatePackStatus`. SwiftUI consumes live frames for immediate list updates, refreshes session summaries, lazy-loads selected request/response bodies by body ref, replays the selected exchange, and has `CoreClient` methods for HAR JSON `Data`, CA, system proxy, and rule-pack operations. The Rules section now provides a native pack list/editor with add/remove, enable toggle, validate, save, and dirty/validation status. The Certificates section now refreshes CA trust status, runs CA install, and changes the install action to `Reinstall` when the CA is already trusted; the System Proxy section now refreshes current macOS HTTP/HTTPS proxy state and enables/disables proxying against the selected port. The UI now has a compact source-list sidebar, flat titlebar proxy controls, fixed titlebar control slots to avoid start/stop jitter, and shared token-backed action button styling. Native `URLSession` system-proxy verification passed on Wi-Fi with a fake local proxy, and Wi-Fi HTTP/HTTPS/bypass settings were restored. Sidecar CA verification passed: status reported missing before install, install returned true, and status reported installed afterward. Sidecar `systemProxy.status` now returns current service states and whether they match the requested Proxyman target.
- Known gaps: Deeper CA trust-state reporting, explicit service selection, authenticated proxy/PAC/SOCKS preservation, and richer rule-engine items remain out of the first shell unless they become blockers.
- Next tasks: move app entry/services/stores toward the planned folder shape, replace the temporary smoke script with `script/build_and_run.sh`, and add narrow SwiftPM tests for store/event behavior.
