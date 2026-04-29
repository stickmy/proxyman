# Test-First Verification Strategy

Every implementation batch starts by adding tests. The tests define the feature contract for that batch and prevent the SwiftUI rewrite from hiding proxy-core regressions.

## Required Flow Per Batch

1. Add or update tests for the batch.
2. Run only the new tests and confirm they fail for the expected reason.
3. Implement the smallest core change needed.
4. Run the batch tests.
5. Run the broader Rust test suite.
6. Record remaining gaps in the batch document before moving on.

## Preferred Test Types

Unit tests:

- Rule parsing and validation.
- Regex compilation.
- SSE parser state machine.
- Header/body preview logic.
- Error mapping.

Integration tests:

- HTTP request and response through proxy.
- HTTPS MITM through proxy.
- Chunked and large streaming responses.
- SSE stream behavior.
- WebSocket echo behavior.
- Replay and storage round trips.

Manual tests:

- macOS CA installation.
- macOS system proxy enable/disable/restore.
- macOS system proxy effect with Swift `URLSession` and a fake local proxy.
- SwiftUI sidecar lifecycle from preferred port `9000`, including launch/status-refresh preflight to the first free port and Start using the displayed port without a second search.
- SwiftUI event socket with at least one UI subscriber and one external `nc -U` subscriber receiving the same `proxyEvent` frames.
- Swift `URLSession` system-proxy effect with `scripts/verify-system-proxy-urlsession.swift` when it is safe to temporarily change macOS network settings.
- SwiftUI visual and interaction checks, including immediate sidebar selection, flat titlebar controls, fixed start/stop control slots, and token-backed button hover styling.

## Naming Convention

Use test names that include the batch or feature area so verification can be scoped:

```text
correctness_preserves_connect_authority_port
streaming_forwards_chunk_before_response_eof
sse_reassembles_event_split_across_chunks
websocket_forwards_text_messages_bidirectionally
rules_reject_invalid_regex_at_validation_time
core_api_rules_parses_simple_line_dsl
rules_pack_persistence_writes_user_authored_rules_file
typed_rules_runtime_redirects_request
rules_typed_block_returns_response_without_upstream
rules_typed_response_header_mutates_matching_response
```

## Verification Commands

Current repository command:

```sh
cargo test --manifest-path src-tauri/Cargo.toml
```

Current full-suite baseline as of 2026-04-29:

- Full suite: 115 passed, 0 failed in the main Rust test targets, plus the sidecar bin test passes.
- `cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar` passes.
- `git diff --check` passes.
- Known warnings: existing dead-code/private-API warnings remain in transition targets.

Current Batch 08 spot-check baseline as of 2026-04-29:

- `cargo test --manifest-path src-tauri/Cargo.toml sidecar_ -- --nocapture` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml core_api_session_ -- --nocapture` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes.
- `cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar` passes.
- `swift build --package-path swiftui/ProxymanClient` passes.
- `git diff --check` passes.
- `scripts/run-swiftui-minimal-test.sh start` launches the SwiftUI client with bundled sidecar.
- `proxy.availablePort` returns the first free loopback port from preferred `9000`; with `*:9000` occupied, the verified return value is `9001`.
- SwiftUI calls `proxy.availablePort` during stopped launch/status refresh and sends `proxy.start` with `findAvailable: false` from the Start action.
- Event socket receives live `proxyEvent` frames while the UI also consumes them.
- Session commands `sessions.search`, `sessions.loadBody`, and `sessions.clear` work against the in-memory sidecar store.
- Replay command `replay.send` replays an in-memory captured request with edits against a local origin.
- HAR commands `har.export` and `har.import` round-trip an in-memory sidecar session.
- CA/system proxy socket validation rejects bad requests before running OS commands.
- Rule-pack socket methods load, validate, save, list, update, and remove a `.rules` pack.
- SwiftUI Rules section builds with pack list/editor, validate, save, add/remove, and enable toggle wired to `CoreClient`.
- SwiftUI Certificates and System Proxy sections build with CA status/install and system-proxy status/enable/disable controls wired to `CoreClient`; the Certificates action label shows `Reinstall` when `ca.status` reports trusted.
- SwiftUI visual smoke checks cover compact sidebar behavior, titlebar proxy controls, fixed titlebar control slots, shared `ProxymanButtonStyle`, and hover background `#F2F2F0`.
- `swiftc -typecheck scripts/verify-system-proxy-urlsession.swift` passes.
- `scripts/verify-system-proxy-urlsession.swift --service Wi-Fi` verifies native `URLSession` HTTP and HTTPS traffic through a fake local proxy and restores the original Wi-Fi proxy settings.
- Sidecar `ca.status`/`ca.install` verification passes on macOS.
- Sidecar `systemProxy.status` returns current macOS HTTP/HTTPS service proxy state without changing system settings.

Batch-scoped examples:

```sh
cargo test --manifest-path src-tauri/Cargo.toml correctness
cargo test --manifest-path src-tauri/Cargo.toml streaming
cargo test --manifest-path src-tauri/Cargo.toml core_api_events
cargo test --manifest-path src-tauri/Cargo.toml exchange_error
cargo test --manifest-path src-tauri/Cargo.toml sse
cargo test --manifest-path src-tauri/Cargo.toml websocket
cargo test --manifest-path src-tauri/Cargo.toml typed_rules_runtime
cargo test --manifest-path src-tauri/Cargo.toml rules_typed_response_header_mutates_matching_response
cargo test --manifest-path src-tauri/Cargo.toml request_header
cargo test --manifest-path src-tauri/Cargo.toml rules_response_header
cargo test --manifest-path src-tauri/Cargo.toml system_proxy_
cargo test --manifest-path src-tauri/Cargo.toml system_proxy -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml ca_
cargo test --manifest-path src-tauri/Cargo.toml session_store
cargo test --manifest-path src-tauri/Cargo.toml core_api_session_
cargo test --manifest-path src-tauri/Cargo.toml sidecar_
cargo test --manifest-path src-tauri/Cargo.toml replay
cargo test --manifest-path src-tauri/Cargo.toml imports_har
swiftc -typecheck scripts/verify-system-proxy-urlsession.swift
swift build --package-path swiftui/ProxymanClient
cargo check --manifest-path src-tauri/Cargo.toml --bin proxyman-sidecar
scripts/run-swiftui-minimal-test.sh start
```

SwiftUI sidecar lifecycle spot check:

```sh
printf '{"jsonrpc":"2.0","id":1,"method":"proxy.availablePort","params":{"host":"127.0.0.1","port":9000}}\n' \
  | nc -U "${TMPDIR%/}/proxyman-swiftui-client/command.sock"
printf '{"jsonrpc":"2.0","id":1,"method":"proxy.start","params":{"host":"127.0.0.1","port":<available-port>,"findAvailable":false}}\n' \
  | nc -U "${TMPDIR%/}/proxyman-swiftui-client/command.sock"
curl --proxy http://127.0.0.1:<returned-port> --noproxy "" http://127.0.0.1:<origin-port>/
```

SwiftUI event-socket spot check:

```sh
nc -U "${TMPDIR%/}/proxyman-swiftui-client/event.sock"
```

Then start the proxy, send one `curl --proxy` request through the returned port, and verify the event stream includes `proxyEvent` frames with `exchangeStarted`, `requestHead`, `responseHead`, `responseFinished`, and `exchangeFinished`.

After the proxy core is extracted, prefer workspace commands:

```sh
cargo test -p proxy-core
cargo test -p proxy-core sse
cargo test --workspace
```

## Completion Record Template

Each batch document should end with a short status block:

```md
## Status

- State: Not started | Tests written | Implementing | Verified | Blocked
- Test command:
- Last verified:
- Known gaps:
```
