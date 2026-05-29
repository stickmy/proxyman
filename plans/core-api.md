# Proxy Core API Direction

The SwiftUI app should not depend on old frontend command names or legacy React event shapes. The Rust proxy core is exposed through a sidecar JSON-RPC command socket and a separate event socket.

## Core Responsibilities

- Start and stop proxy listeners.
- Emit exchange lifecycle events.
- Manage rules and rule packs.
- Manage CA generation and trust state.
- Manage macOS system proxy state.
- Query the current in-memory session and explicitly import/export HAR.
- Replay requests.

## Current Sidecar Surface

These APIs exist today on the Unix domain socket sidecar:

- Proxy lifecycle: `proxy.status`, `proxy.availablePort`, `proxy.start`, `proxy.stop`.
- CA: `ca.status`, `ca.install`.
- System proxy: `systemProxy.status`, `systemProxy.enable`, `systemProxy.disable`.
- Rules: `rules.listPacks`, `rules.getPackRules`, `rules.validate`, `rules.savePackRules`, `rules.addPack`, `rules.removePack`, `rules.updatePackStatus`.
- Sessions: `sessions.search`, `sessions.loadBody`, `sessions.clear`, `replay.send`, `har.export`, `har.import`.
- Typed proxy controller: `StandaloneProxyController` owns sidecar proxy lifecycle and rejects ephemeral port `0` for stable UI-facing starts.
- Typed event facade: `core_api::events` maps internal proxy events into `proxy_event_v1` envelopes for exchange started, request heads, request body chunks from captured request bodies, request finished, response heads, response finished, exchange finished, SSE events, and WebSocket messages. Network response streaming also emits typed-only `ResponseBodyChunk` events through a separate runtime event channel. Request/response body chunks and WebSocket payload previews include UTF-8 vs lossy UTF-8 metadata, WebSocket close frames include code/reason metadata, request/response decode failures emit typed-only `ExchangeError` events, and the typed runtime event channel uses an explicit non-blocking `DropNewest` backpressure policy.
- Typed CA facade: `core_api::ca` defines versioned status/install request and response structs over the current certificate commands.
- Typed system proxy facade: `core_api::system_proxy` defines versioned enable/disable request and response structs and rejects port `0` for stable UI-facing enables.
- Typed rule facade: `core_api::rules` defines versioned rule content and rule pack request/response structs, plus a stable `RuleKind` enum over the current processor modes. It also has typed rule contract DTOs, `validate_typed_rules_v1`, and a lightweight user-authored `.rules` parser/formatter for redirect, delay, request-header, response-header, block, map-remote, map-local, request-body, and response-body rules. `save_rule_pack_rules_v1` persists the original user-authored text after validation and clears old per-processor files in that pack. `TypedRuleProcessor` applies compatible actions in order, including request-side redirect, delay, request-header, request-body, map-remote, map-local, and block, plus response-side response-header, response-body, delay, and typed response block with request URL and status matching.
- Typed session facade: `core_api::session::CoreSessionApi` delegates search/body/HAR/replay to a typed `SessionStore` contract. First SwiftUI shell uses an in-memory `SessionStore`; JSONL is transition support, not the native-client storage contract.

The active API direction is to keep strict module-level facades inside `crates/proxyman-core` first, then split smaller Rust crates only once the contracts stop moving. SwiftUI should depend on the sidecar/IPC contract, not on old React-era event shapes.

## Current Facade Decisions

- Start with a module-level facade instead of extracting a crate immediately.
- Use Unix domain sockets for the first native sidecar boundary: one newline-delimited JSON-RPC command socket and one newline-delimited event socket.
- Do not expose raw JSONL event values through new SwiftUI-facing APIs. The session facade now uses typed `SessionStore` operations for search/body/HAR/replay; only HAR import/export intentionally uses `serde_json::Value`.
- Keep `serde_json::Value` only at deliberate import/export boundaries such as HAR.
- Version new facade request/response structs with `apiVersion`.
- Use in-memory session storage for the first SwiftUI shell. App quit drops captured exchanges unless the user explicitly exports HAR.

## Facade Progress Matrix

| Area | Stable enough for SwiftUI prototype | Current shape | Remaining work |
| --- | --- | --- | --- |
| Proxy lifecycle | Yes | `StandaloneProxyController`; sidecar JSON-RPC exposes `proxy.status`, `proxy.availablePort`, `proxy.start`, and `proxy.stop` | Add diagnostics/health details and keep lifecycle semantics stable while adding more sidecar methods |
| Events | Yes for prototype | `proxy_event_v1` typed envelopes for lifecycle, request/response heads, body chunks, SSE, WebSocket text/binary/control messages, decode errors, and `DropNewest` backpressure; sidecar event socket broadcasts `proxyEvent` frames to SwiftUI and external subscribers; sidecar also stores typed `proxyEvent` frames in the in-memory session store | Add reconnect/backfill refinements using current in-memory exchange summaries |
| Sessions | Yes for prototype | `search_session_exchanges`, lazy body, replay, HAR import/export over a typed `CoreSessionApi<SessionStore>` contract; `InMemorySessionStore` backs the SwiftUI sidecar and exposes `sessions.search`, `sessions.loadBody`, `sessions.clear`, `replay.send`, `har.export`, and `har.import` | Keep JSONL only as transition/legacy support; add HTTPS replay parity and richer HAR phases when needed |
| Rules | Partially | `core_api::rules` transition DTOs over current processor commands and packs, typed rule DTO/validation through `validate_typed_rules_v1`, `.rules` parser/formatter/persistence, request-side runtime for redirect/delay/request-header/request-body/map-remote/map-local/block, response-side runtime for response-header/response-body/delay/typed block, sidecar JSON-RPC for `rules.getPackRules`, `rules.validate`, `rules.savePackRules`, `rules.addPack`, `rules.removePack`, and `rules.updatePackStatus`, and first SwiftUI pack list/editor controls | Fault actions, body-preview matcher execution, richer breakpoint-style response actions, rule-hit event details |
| CA | Partially | Sidecar JSON-RPC exposes `ca.status` and `ca.install`; SwiftUI Certificates controls are wired and sidecar status/install passed on macOS | Rich trust state: installed/missing/stale |
| System proxy | Partially | Sidecar JSON-RPC exposes `systemProxy.status`, `systemProxy.enable`, and `systemProxy.disable`; native `URLSession` verification passed on Wi-Fi | Explicit service selection and PAC/SOCKS/auth proxy preservation |

## Typed Event Progress

Implemented in `proxy_event_v1`:

- `ExchangeStarted`
- `RequestHead`
- `RequestBodyChunk` from captured request bodies
- `RequestFinished`
- `ResponseHead`
- `ResponseBodyChunk` from the runtime streaming path
- `ResponseFinished`
- `ExchangeFinished`
- `ExchangeError` for request/response decode failures
- `SseEvent`
- `WebSocketMessage` for text, binary, ping, pong, and close frame capture with stable preview metadata

Event stream rules:

- `proxyEvent` is the SwiftUI-facing typed stream over the event socket.
- Runtime response chunks and exchange errors are captured in the sidecar in-memory session model, not a legacy frontend stream.
- Body chunk and WebSocket payload previews include `previewEncoding`, `lossyPreview`, and truncation metadata; they are not full body storage.
- The typed runtime channel uses non-blocking `DropNewest` backpressure.

First native event framing:

- Commands: newline-delimited JSON-RPC 2.0 over Unix domain socket.
- Events: newline-delimited `proxy_event_v1` envelopes over a separate Unix domain socket.
- Socket paths are per app run and live in the app runtime/data directory.
- Reconnect should resubscribe to the live stream and then query current in-memory exchange summaries.

## Suggested Event Model

Events should be structured and incremental:

```text
ExchangeStarted
RequestHead
RequestBodyChunk
RequestFinished
ResponseHead
ResponseBodyChunk
SseEvent
WebSocketMessage
ResponseFinished
ExchangeFinished
ExchangeError
```

Each event should include:

- `exchange_id`
- timestamp
- direction where applicable
- byte offsets for body chunks
- capture metadata
- rule effects where applicable

## SwiftUI Integration Direction

Start with a process or IPC boundary instead of direct FFI unless performance proves otherwise.

Preferred first shape:

- Rust proxy core runs as a local service or sidecar.
- SwiftUI talks to it through newline-delimited JSON-RPC over Unix domain socket.
- Event stream is separate from command requests.
- Body data is lazy-loaded by ID or file reference.

Reasons:

- Proxy failures do not crash the UI.
- Streaming events and backpressure are easier to reason about.
- The same core can later support CLI/headless mode.

## First SwiftUI API Surface

The first native client depends on these methods/events only:

- `proxy.start`, `proxy.stop`, `proxy.status`
- `proxy.availablePort`
- `events.subscribe` through the event socket
- `sessions.search`, `sessions.loadBody`, `sessions.clear`
- `replay.send`
- `har.export`, `har.import`
- `rules.getPackRules`, `rules.validate`, `rules.savePackRules`, `rules.addPack`, `rules.removePack`, `rules.updatePackStatus`
- `ca.status`, `ca.install`
- `systemProxy.status`, `systemProxy.enable`, `systemProxy.disable`
- `diagnostics.status`

See [swiftui-client.md](swiftui-client.md) for the decision details.

## Pre-SwiftUI API Freeze Checklist

- Define command structs for proxy lifecycle, system proxy, CA status/install, rule CRUD, session search, body loading, replay, and HAR import/export. Proxy lifecycle, system proxy, CA, rule content/pack operations, and session search/body/HAR/replay now have first typed structs. Proxy lifecycle is also exposed through the first Unix-socket sidecar methods, including available-port preflight.
- Define event structs for exchange lifecycle, request/response heads, body chunks, SSE events, WebSocket messages, completion, and errors. Request heads, response heads, request/response body chunks, SSE, WebSocket text/binary/control messages, basic lifecycle completion, exchange errors, body chunk and WebSocket payload preview metadata, close frame metadata, and explicit `DropNewest` backpressure policy now have typed event/API coverage.
- Move remaining storage operations behind traits or module facades so the first SwiftUI client can use in-memory storage without changing UI code. Session search/body/HAR/replay now goes through a typed `SessionStore` facade, and the first sidecar in-memory store covers search/body/clear/replay/HAR.
- Expose session search/body/HAR/replay over the Unix-socket sidecar before the SwiftUI client depends on them. Search/body/clear/HAR/replay are exposed.
- Avoid exposing `serde_json::Value` in SwiftUI-facing APIs except at a deliberate import/export boundary such as HAR.
- Add explicit version fields for exported/imported sessions and rule packs before writing a native UI around them.

## Adapter Rule

During migration:

- Do not add new business logic to the removed frontend.
- New behavior belongs in the proxy core or system integration layer.
