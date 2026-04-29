# Proxy Core Rewrite Plans

This directory tracks the migration from the current Tauri/React proxy app to a SwiftUI desktop app backed by a stable Rust proxy core.

The old frontend is treated as disposable. It can be used as a reference for workflows, but it should not block proxy-core work.

## New Session Startup

When continuing this project in a new session, start here:

1. Read this file first.
2. Read [batches.md](batches.md) for detailed batch status.
3. Read [core-api.md](core-api.md) before changing API, IPC, command, event, or SwiftUI-facing surfaces.
4. Read [swiftui-client.md](swiftui-client.md) before changing native-client, IPC, system-proxy verification, or first-shell storage decisions.
5. Read [rules-format.md](rules-format.md) before changing user-authored rule configuration.
6. Read [session-storage-backend.md](session-storage-backend.md) before changing session persistence backends.
7. Read [testing.md](testing.md) before adding implementation.

Default next instruction for a new session:

```text
先读 plans/README.md，再读 plans/batches.md、plans/core-api.md 和 plans/swiftui-client.md，继续按 Next Work 做。第一版 SwiftUI 客户端走 Unix domain socket sidecar 方向，先用 mock CoreClient 启动 UI 骨架；真实 sidecar/IPC 按文档里的 command socket + event socket 落地。每批先写测试，确认失败，再实现并更新文档。
```

## Execution Rules

1. Start each batch by writing or updating tests first.
2. Run the new tests and confirm they fail for the expected reason before implementation.
3. Implement only the batch scope.
4. Run the batch verification commands.
5. Update the batch document with status, known gaps, and follow-up items.

## Batch Order

| Batch | Area | Goal | Status | Verification Boundary |
| --- | --- | --- | --- | --- |
| 00 | Test harness and core boundary | Make proxy behavior testable outside UI/Tauri | Verified | Unit and integration harness runs locally |
| 01 | Correctness hardening | Remove known panics and preserve HTTP authority semantics | Verified | Bad input tests pass without crashing |
| 02 | Streaming capture foundation | Forward bodies while capturing bounded chunks | Verified | Large body and chunked response tests pass |
| 03 | SSE | Parse and expose server-sent events incrementally | Verified | SSE fixture stays open and emits events |
| 04 | WebSocket | Bidirectional proxy and message-level capture | Verified | Echo server tests pass through proxy |
| 05 | Rule engine | Typed matchers, compiled rules, request/response mutation | Partially verified | Rule validation and mutation tests pass |
| 06 | CA and system proxy | Per-user CA and robust macOS proxy state | Partially verified | macOS integration tests/manual checklist pass |
| 07 | Storage, replay, export | Persist sessions and support replay/HAR | Partially verified | Storage/replay/HAR tests pass |
| 08 | SwiftUI shell | Native macOS UI over stable core API | In progress | Minimal SwiftUI client drives sidecar lifecycle and live events |

See [batches.md](batches.md) for detailed scope and [testing.md](testing.md) for the test-first workflow.

## Current Progress

Last verified:

- Full Rust suite baseline: 2026-04-29.
- Batch 08 sidecar/client spot checks: 2026-04-29.

Current full-suite baseline:

```sh
cargo test --manifest-path src-tauri/Cargo.toml
```

Current result: 115 passed, 0 failed in the main Rust test targets, plus the sidecar bin tests pass. `cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar` also passes, with existing dead-code warnings still present.

Current Batch 08 spot-check result:

- `cargo test --manifest-path src-tauri/Cargo.toml sidecar_ -- --nocapture` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml core_api_session_ -- --nocapture` passes.
- `cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar` passes.
- `swift build --package-path swiftui/ProxymanClient` passes.
- `git diff --check` passes.
- `sidecar_har_export_import_round_trips_runtime_store` passes inside the `sidecar_` group.
- `sidecar_replay_send_replays_in_memory_request_with_edits` passes inside the `sidecar_` group.
- `sidecar_rules_pack_commands_use_stable_socket_api` passes inside the `sidecar_` group.
- `sidecar_rejects_invalid_system_proxy_enable_before_networksetup` and `sidecar_rejects_invalid_ca_status_version_before_security_command` cover bounded CA/system proxy socket validation without changing OS state.
- `swiftc -typecheck scripts/verify-system-proxy-urlsession.swift` passes.
- `scripts/verify-system-proxy-urlsession.swift --service Wi-Fi` verifies native `URLSession` HTTP absolute-form and HTTPS `CONNECT` traffic through a fake local proxy, then restores the original Wi-Fi HTTP/HTTPS proxy and bypass-domain settings.
- Sidecar `ca.status` returned `installed:false`, `ca.install` returned `installed:true`, and a follow-up `ca.status` returned `installed:true` on this machine.
- `scripts/run-swiftui-minimal-test.sh start` launches the temporary SwiftUI app with a bundled `proxyman-sidecar`.
- With another process listening on `*:9000`, `proxy.availablePort` returns `9001`; the SwiftUI port field also refreshes to `9001`.
- Lower-level `proxy.start` with `findAvailable: true` binds the first free loopback port and forwards a local HTTP request through `curl --proxy`; the SwiftUI Start action preflights the port earlier and sends `findAvailable: false`.
- The Unix event socket broadcasts `proxyEvent` frames to SwiftUI and an external `nc -U` subscriber.

Implemented core capabilities:

- Proxy behavior can be tested without relying on the old React UI.
- Known panic paths around malformed rules, bad redirect destinations, unsupported request encoding, CONNECT custom ports, and double stop have regression tests.
- Response forwarding is streaming-first for chunked and large responses; capture is bounded.
- SSE streams are parsed incrementally and emitted as structured events.
- WebSocket text, binary, ping, pong, and close frames are forwarded through the proxy and captured with stable preview metadata.
- Rules now validate regex at build/update time, use user-authored `.rules` packs for typed rules, and execute typed request-side redirect/delay/request-header/request-body/map-remote/map-local/block plus response-side response-header/response-body/delay and typed response block.
- CA material is generated per local app data path; the old bundled shared CA key/cert were removed.
- macOS system proxy enable/disable now handles enabled services instead of hardcoding `Wi-Fi`, and snapshots/restores HTTP/HTTPS proxy and bypass domains.
- Sessions currently have JSONL transition support with sidecar large bodies, searchable by basic filters, replayable by exchange id, and import/export basic HAR 1.2. First SwiftUI shell now has an in-memory sidecar session store fed by typed `proxyEvent` frames, with `sessions.search`, `sessions.loadBody`, `sessions.clear`, `replay.send`, `har.export`, and `har.import` over the Unix command socket.
- First strict module-level core API facades now exist for proxy lifecycle, typed proxy events, CA status/install, macOS system proxy enable/disable, rule content/pack operations, and session search/body/HAR/replay. They keep old Tauri commands as compatibility adapters while exposing versioned typed boundaries for future SwiftUI/IPC work.
- Batch 08 now has a runnable minimal SwiftUI client that owns a bundled Rust sidecar, talks over Unix sockets, starts/stops the real proxy core, preflights the first available port, updates its capture list from live typed proxy events, refreshes session summaries, lazy-loads selected request/response bodies by body ref, edits rule packs, and exposes HAR, CA, and system-proxy controls through `CoreClient`.

Current core API progress:

- `core_api::proxy`: versioned start/stop/status structs and `*_v1` Tauri adapters; the sidecar exposes lifecycle JSON-RPC methods plus `proxy.availablePort` for SwiftUI port preflight.
- `core_api::events`: `proxy_event_v1` envelopes for exchange lifecycle, request/response heads, request body chunks from captured request bodies, runtime response body chunks, SSE, WebSocket messages with preview metadata, decode errors, and typed-event backpressure.
- `core_api::session`: typed `SessionStore` facade for exchange-summary search, lazy body loading, replay, and HAR import/export, with JSONL kept as transition support and filters for method, host, path, status, body text, header name, and header value. The first SwiftUI shell uses an `InMemorySessionStore` behind the same facade for search/body/clear/replay/HAR import/export.
- `core_api::rules`: transition facade over current processor pack commands using versioned DTOs and stable `RuleKind`; typed rule contract DTOs and `validate_typed_rules_v1` now cover rule ids, priorities, request/response phases, typed matchers, typed actions, regex validation, phase validation, and deterministic evaluation order. The user-authored `.rules` line format parses/formats into the typed DTO layer for redirect, delay, request-header, response-header, block, map-remote, map-local, request-body, and response-body rules, and rule packs now persist the original user text as `rules.rules`. Runtime execution applies compatible typed actions in order, including request-side redirect, delay, request-header, request-body, map-remote, map-local, and block, plus response-side response-header, response-body, delay, and typed response block with request URL and status matching.
- `core_api::ca` and `core_api::system_proxy`: versioned status/install and enable/disable DTOs over the current macOS integration commands.
- `swiftui/ProxymanClient`: first SwiftPM macOS SwiftUI client skeleton with proxy status controls, capture list, exchange detail preview, rule editor, CA trust controls, and system-proxy controls. It can use a mock `CoreClient` or a bundled `proxyman-sidecar`; lifecycle, available-port preflight, session search/body loading, session clear, replay, HAR import/export, CA, system proxy, and rule-pack methods use the Unix command socket, while live capture-list updates consume `proxyEvent` frames from the Unix event socket.

Current event-stream contract:

- Old React/Tauri compatibility stream: `proxy_event`, unchanged, still carries legacy events and is persisted to JSONL.
- New SwiftUI-facing stream: `proxy_event_v1`, versioned typed envelopes, includes lifecycle/chunk/error events.
- Runtime response body chunks and exchange errors are typed-only and are not written to the JSONL session log.
- Body and WebSocket payload previews include `previewEncoding`, `lossyPreview`, and truncation metadata; WebSocket close frames also expose close code/reason.
- Typed event runtime channel is non-blocking and uses `DropNewest` when full, so UI/event sink slowness does not block network forwarding.

Current native-client decisions:

- SwiftUI talks to the Rust core sidecar over Unix domain sockets.
- Command traffic uses newline-delimited JSON-RPC 2.0 on one socket.
- Event traffic uses newline-delimited typed event envelopes on a separate socket.
- SwiftUI uses preferred proxy port `9000`, preflights the first free loopback port with `proxy.availablePort` on launch/refresh, and starts the already displayed port with `findAvailable: false`.
- First shell storage is in-memory by default; app quit drops captures unless the user exports HAR.
- System-proxy manual verification uses Swift `URLSession` plus a fake local proxy, not plain `curl`.

## Next Work

Current phase: Batch 08 has passed lifecycle/event/session/HAR/replay socket smoke tests and now has bounded CA/system proxy/rules socket coverage. SwiftUI rule-pack editing, CA trust controls, and system-proxy controls are wired to the existing socket API. Native `URLSession` system-proxy verification, sidecar CA install/status verification, read-only `systemProxy.status` verification, and multiple visual SwiftUI smoke passes have also passed on macOS.

1. Move app entry/services/stores toward the planned folder shape and keep the view files focused.
2. Replace the temporary smoke path with `script/build_and_run.sh` and Codex environment metadata.
3. Keep deeper CA trust-state reporting, service selection, PAC/SOCKS/auth proxy preservation, and richer rule-engine deferrals out of the first shell unless they become UI blockers.

Batch 08 completed so far:

1. SwiftUI app skeleton with native split view, proxy controls, capture list, and detail placeholder.
2. Bundled `proxyman-sidecar` launch from the SwiftUI app.
3. Unix command socket for `proxy.status`, `proxy.availablePort`, `proxy.start`, and `proxy.stop`.
4. Unix event socket with multiple subscribers and live `proxyEvent` capture-list updates.
5. First available-port behavior from preferred port `9000`, including the occupied-9000 fallback to `9001`.
6. In-memory `SessionStore` behind the sidecar, fed by typed `proxyEvent` frames.
7. Unix command socket methods for `sessions.search`, `sessions.loadBody`, and `sessions.clear`.
8. SwiftUI session summary refresh and selected-exchange request/response body lazy loading by body ref.
9. Unix command socket methods for `har.export` and `har.import`, verified with an in-memory round trip.
10. SwiftUI `CoreClient` HAR export/import methods using JSON `Data` at the UI boundary.
11. Unix command socket method `replay.send`, verified against an in-memory captured exchange and a local origin.
12. SwiftUI `CoreClient` replay method and action button for the selected exchange.
13. Manual native `URLSession` system-proxy verification script at `scripts/verify-system-proxy-urlsession.swift`.
14. Unix command socket methods for `ca.status`, `ca.install`, `systemProxy.enable`, `systemProxy.disable`, and rule-pack load/validate/save/add/remove/status operations.
15. SwiftUI `CoreClient` methods for CA, system proxy, and rule-pack operations.
16. SwiftUI Rules section with pack list, add/remove, enable toggle, validate, save, and `.rules` editor.
17. SwiftUI Certificates and System Proxy sections with CA status/install and system proxy enable/disable controls. The certificate action now reads `Reinstall` when the CA is already trusted.
18. Read-only `systemProxy.status` command and SwiftUI System Proxy status refresh using current macOS HTTP/HTTPS proxy state.
19. Compact source-list sidebar with immediate visual selection and no invented workspace concept.
20. Flat titlebar proxy controls with fixed slots for status, endpoint, refresh, and start/stop so state changes do not shift layout.
21. Shared token-backed `ProxymanButtonStyle` for action buttons: matching background, light gray border, 8 pt radius, and `#F2F2F0` hover fill.
