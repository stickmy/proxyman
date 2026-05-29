# Session Storage Backend Decision

Status: first native shell uses in-memory sessions by default. Do not switch to SQLite before the first native shell unless the cutover criteria below become true.

The SwiftUI-facing API must depend on `core_api::session::CoreSessionApi` and the typed `SessionStore` contract, not on JSONL event records. JSONL remains transition support for local debugging and migration, but it is not the first SwiftUI storage model.

## First Native Model

- App launch creates a new empty in-memory session.
- Captured exchanges, body previews, and body references live in memory unless the user explicitly exports HAR.
- Quitting or killing the app drops captured traffic.
- HAR import loads entries into the current in-memory session.
- HAR export is the first persistence/export path.
- Replay reads from the current in-memory session.

This matches the expected debugging workflow: live captures are temporary unless the user saves them.

## Current Transition Implementation

- Legacy and transition capture writes append-only JSONL events under app data.
- Large bodies spill to sidecar files and are loaded lazily by `bodyRef`.
- `JsonlSessionStore` converts raw event records into typed exchange summaries behind `SessionStore`.
- HAR import/export intentionally remains a `serde_json::Value` boundary because HAR is itself JSON.

## Cutover Criteria

Move to indexed SQLite before SwiftUI only if the first native shell requires one or more of these:

- Fast search over tens of thousands of exchanges in a single session.
- Stable pagination, sorting, and time-range filtering for long history.
- Tags, pinned state, notes, or retention policy in the first native workflow.
- Header/body preview indexing or full-text search.
- Multi-session history across restarts with migration and schema version guarantees.

If these are not first-shell requirements, keep JSONL and implement SQLite later as another `SessionStore`.

## In-Memory Store Shape

The first native store should keep:

- Exchange summaries keyed by exchange id.
- Request and response heads.
- Bounded request/response body chunks or references.
- SSE/WebSocket child events linked to the parent exchange.
- Rule-effect metadata attached to the exchange.

The store should expose the same `SessionStore` operations as JSONL: search, load body, replay source lookup, HAR export, HAR import, and clear current session.

## Minimum SQLite Shape

Do not treat this as final schema; it is a compatibility target for the existing facade.

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  label TEXT
);

CREATE TABLE exchanges (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  method TEXT,
  uri TEXT,
  scheme TEXT,
  host TEXT,
  port INTEGER,
  path TEXT,
  status INTEGER,
  request_time INTEGER,
  response_time INTEGER,
  request_body_ref TEXT,
  response_body_ref TEXT,
  request_body_preview TEXT,
  response_body_preview TEXT
);

CREATE TABLE headers (
  exchange_id TEXT NOT NULL REFERENCES exchanges(id),
  phase TEXT NOT NULL,
  name TEXT NOT NULL,
  value TEXT NOT NULL
);

CREATE INDEX idx_exchanges_session_time ON exchanges(session_id, request_time);
CREATE INDEX idx_exchanges_host_path ON exchanges(host, path);
CREATE INDEX idx_exchanges_method_status ON exchanges(method, status);
CREATE INDEX idx_headers_name_value ON headers(name, value);
```

Likely later additions: `tags`, `exchange_tags`, `notes`, `rule_hits`, `websocket_messages`, `sse_events`, retention metadata, and FTS tables for previews.

## Migration Rules

- Keep `SessionStore` as the only SwiftUI-facing storage boundary.
- Implement `InMemorySessionStore` before wiring SwiftUI to the real sidecar.
- Add a schema version table before writing production SQLite files.
- Preserve body sidecars unless there is a measured reason to move bodies into SQLite blobs.
- Keep JSONL import available at least as a migration/debugging path while the storage model settles.
