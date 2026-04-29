# User Rule Format

Rules are user-authored text files with one rule per line. The file format is intentionally smaller than TOML/YAML: a command, a match column, a URL glob, then action-specific arguments.

## Current Shape

```text
# action          match  url glob                         args
redirect          GET    https://api.example.com/users*    https://mock.local/users
delay             *      https://api.example.com/slow*     500ms
request-header    GET    https://api.example.com/*         x-debug true
response-header   200    https://api.example.com/*         x-cache hit
block             *      https://ads.example.com/*         403
map-remote        GET    https://api.example.com/*         https://upstream.example.com
map-local         *      https://static.example.com/*      /tmp/static.json
request-body      POST   https://api.example.com/users*    patched request body
response-body     201    https://api.example.com/users*    patched response body
```

Blank lines and `#` comments are ignored.

## Columns

| Action | Match column | Args |
| --- | --- | --- |
| `redirect` | HTTP method or `*` | destination URL |
| `delay` | HTTP method or `*` | duration such as `500ms` |
| `request-header` | HTTP method or `*` | header name and value |
| `response-header` | HTTP status or `*` | header name and value |
| `block` | HTTP method or `*` | response status, optionally followed by body text |
| `map-remote` | HTTP method or `*` | destination URL |
| `map-local` | HTTP method or `*` | local file path |
| `request-body` | HTTP method or `*` | replacement body text |
| `response-body` | HTTP status or `*` | replacement body text |

URL globs support `*` wildcard matching:

```text
https://api.example.com/users*
https://*.example.com/*
http://127.0.0.1:8080/*
*
```

When a URL glob includes an explicit port, the parser stores it as a separate typed port matcher. Runtime matching compares scheme, host, port, and path separately instead of treating `host:port` as a single host string.

## Runtime Support

Currently executed from `.rules` packs:

| Action | Runtime phase | Status |
| --- | --- | --- |
| `redirect` | Request | Implemented |
| `delay` | Request | Implemented |
| `request-header` | Request | Implemented |
| `block` | Request | Implemented |
| `map-remote` | Request | Implemented |
| `map-local` | Request | Implemented |
| `request-body` | Request | Implemented |
| `response-header` | Response | Implemented with request URL and response status matching |
| `response-body` | Response | Implemented with request URL and response status matching |
| typed `Block` | Response | Implemented through typed API with request URL and response status matching |

Not implemented yet:

- fault injection
- body-preview matchers
- richer fault and breakpoint-style response actions

## Internal Contract

The `.rules` file is only the editing format. It parses into typed rule DTOs in `core_api::rules`, then validation applies the same phase, matcher, action, regex, status, and header checks as the SwiftUI-facing typed API. A single typed rule can contain multiple actions; runtime applies compatible actions in order and stops when an action returns a synthetic response.

Rule packs persist the original user text at:

```text
rule/<pack-name>/rules.rules
```

Saving through `save_rule_pack_rules_v1` validates the file first, preserves comments and spacing in the saved content, writes the pack enabled state, and removes old per-processor files in the same pack.

`TypedRuleProcessor` loads `rules.rules` at startup. Typed rules run before legacy per-processor files in the same pack; saving through the typed pack API removes those legacy files so a pack has one user-authored source of truth.

There is no legacy config-file compatibility requirement. The app is still in development, so persisted rule packs should be written in this format.
