# macOS App Best Practices Plan

This plan treats `swiftui` as the macOS app surface. The old React/Tauri frontend has been removed and should not guide new UI, app structure, or runtime decisions.

## Principles

- Keep the SwiftUI client native, compact, and desktop-first instead of recreating the old React/Tauri interface.
- Keep SwiftUI as the source of truth for value state, selection, commands, and layout.
- Use AppKit only for narrow platform gaps such as cursor behavior, local scroll tuning, text-system control, panels, responder-chain access, or window behavior SwiftUI cannot express cleanly.
- Prefer stable split layouts and explicit selection over top-level root swapping.
- Keep build, run, and visual inspection reproducible through one project-local command.
- Preserve behavior during structural refactors; make visual changes only after file boundaries are clear.

## Current Shape

- SwiftPM executable package: `swiftui`.
- Main app entry: `ProxymanClientApp`.
- Root UI lives in `ContentView.swift`; capture, rules, system proxy, certificates, and shared workspace chrome live in dedicated flat source files under `swiftui/Sources/ProxymanClient/`.
- Value models live in `Models.swift`; app state lives in `AppModel.swift`; the `CoreClient` protocol/mock lives in `CoreClient.swift`; Unix socket transport, sidecar launch, and response DTOs live in `UnixSocketCoreClient.swift`.
- Shared support modifiers and styles currently live as flat files beside the views, including cursor affordances and `ProxymanButtonStyle`.
- Build/run smoke path is `script/build_and_run.sh`; it builds the Rust sidecar, builds the SwiftPM app, stages a local `.app` bundle, embeds the sidecar, and launches the app.
- SwiftPM logic tests live under `swiftui/Tests/ProxymanClientTests/`.
- Manual system-proxy verification lives at `script/verify-system-proxy-urlsession.swift`; it uses Swift only to exercise Foundation `URLSession`, not as a general scripting preference.
- Codex Run action is configured in `.codex/environments/environment.toml`.

## UI Decisions

- Keep the sidebar compact and source-list-like. Do not add product concepts that do not exist in the app model, such as workspaces.
- Sidebar rows should stay lightweight: one icon, one label, 30 pt row height, subtle selected fill, and immediate visual selection on mouse down.
- Do not put back/forward buttons, sidebar toggles, or other unused chrome in the sidebar title area.
- Keep proxy status, host, port, refresh, and start/stop controls visually in the titlebar area. Use a flat SwiftUI overlay instead of system toolbar content when the toolbar draws an unwanted capsule or shadow.
- Give titlebar controls fixed slots so state text and start/stop icon changes do not shift neighboring controls.
- Keep content headers independent from the titlebar controls; the titlebar row should not push the sidebar or workspace layout down more than the intended top inset.
- Reduce line-heavy framing. Use weak split boundaries only where panes need spatial separation, and prefer subtle background changes for list/editor/detail distinction.
- Treat navigation rows and selectable list rows differently from action buttons. Sidebar items and rule pack rows keep plain row selection styling.

## Button And Theme Tokens

- Use `ProxymanButtonStyle` for action buttons.
- Button defaults: transparent background matching the surrounding surface, light gray border, 8 pt continuous corner radius.
- Button hover background is `#F2F2F0`.
- Icon-only action buttons should use the same visual treatment with a fixed square hit target.
- Keep destructive actions visually distinct through foreground color, but keep the same base button geometry.
- SwiftUI and AppKit do not provide a CSS-variable-style design token system. Use a project token layer for app-specific values.
- Preferred token stack:
  - System semantic values first: `Color.primary`, `Color.secondary`, `NSColor.separatorColor`, materials, and control colors.
  - Asset Catalog named colors when design values need Light/Dark variants.
  - Small project theme namespaces such as `ProxymanTheme.Button` for radius, border, hover fill, spacing, and component sizing.
  - Custom `EnvironmentKey` only when runtime theme switching or per-window theme injection is actually needed.

## Runtime Interaction Rules

- Selection changes must update visual state immediately and must not wait for network, socket, or sidecar synchronization.
- Sidecar calls that can block must stay off the main actor path. Keep UI state updates on `@MainActor`, but let client operations run asynchronously behind that boundary.
- Resolve the available proxy port during startup/status refresh while the proxy is stopped.
- Starting the proxy should use the already displayed port and should not run a second available-port search that changes the value after the user clicks Start.
- Stop should preserve the last effective host and port in the UI unless a later refresh discovers a different stopped-state available port.

## Script Decisions

- Keep project-local automation under `script/`.
- Use shell for build, run, stop, logs, and app-bundle staging.
- Use Swift scripts only when the verification target is a native Apple framework behavior that shell/JS cannot faithfully exercise.
- `verify-system-proxy-urlsession.swift` exists because it checks whether Foundation `URLSession` observes macOS HTTP/HTTPS system proxy settings. `curl` is still useful for proxy-core smoke tests, but it does not prove native app traffic uses the same path.

## Target File Shape

- Keep the SwiftPM package root at `swiftui`.
- Keep the first-shell source files flat under `swiftui/Sources/ProxymanClient/`.
- Add subdirectories only when there is enough weight to justify them: `Models/`, `Stores/`, `Services/`, or `Views/` should represent real ownership, not one-file folders.
- Keep files named after their primary type.

## Work Plan

- [x] Capture this project-specific macOS app plan.
- [x] Start the structure pass by extracting shared UI and AppKit cursor helpers from `ContentView.swift`.
- [ ] Move the app entry into `App/` and keep scene/command setup small.
- [x] Split capture workspace into a dedicated view file.
- [x] Split rules workspace into a dedicated view file.
- [x] Split system proxy and certificates into dedicated view files.
- [x] Apply first native macOS UI pass with a controlled split sidebar, source-list rows, and a detail header.
- [x] Move proxy status, endpoint, refresh, and start/stop controls into flat titlebar-style chrome.
- [x] Add fixed titlebar control slots to prevent start/stop layout jitter.
- [x] Add shared action button styling and project-local UI tokens.
- [x] Resolve available proxy port during startup/status refresh instead of only during Start.
- [x] Move pure models and enums out of the observable app store file.
- [ ] Split the observable app state into focused stores or coordinators where the dependency boundary is clear.
- [x] Move sidecar launch and Unix socket transport out of `Models.swift` into a dedicated client/service file.
- [x] Replace the ad hoc SwiftUI smoke script with `script/build_and_run.sh`.
- [x] Add `.codex/environments/environment.toml` once the run script is stable.
- [x] Remove the old React/Tauri frontend and Tauri app-shell code while preserving the Rust proxy core and sidecar.
- [x] Move the Rust proxy core and sidecar crate out of the old app-shell directory into `crates/proxyman-core`.
- [x] Flatten the SwiftPM package root to `swiftui` and remove one-file SwiftUI source subfolders.
- [x] Merge project scripts into `script/`.
- [x] Add narrow SwiftPM tests for sidecar response parsing, event application, and AppModel state behavior before changing those code paths.
- [ ] Run a visual smoke pass after structural changes compile.

## AppKit Use Rules

- Wrap AppKit behavior in small `NSViewRepresentable`, modifier, or service types.
- Do not pass `NSView`, `NSWindow`, or `NSCursor` through feature view hierarchies.
- Keep cursor and text-system bridges narrow and local; move them into `Support/` only when the flat source list becomes too noisy.
- Keep scroll-view tuning local to the specific SwiftUI surface that needs it.
- Re-check AppKit bridges after moving views, because SwiftUI may recreate representables and modifiers.

## Build And Verification

- Use `swift build --package-path swiftui` after each structural slice.
- Once `script/build_and_run.sh` exists, prefer it for build, launch, logs, and visual smoke checks.
- The Rust proxy-core sidecar crate now lives under `crates/proxyman-core`; it no longer depends on Tauri or compiles the Tauri app shell.
- Do not run broad formatters as part of these refactors.
