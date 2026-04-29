# macOS App Best Practices Plan

This plan treats `swiftui/ProxymanClient` as the macOS app surface. The old Tauri frontend is deprecated and should not guide new UI, app structure, or runtime decisions.

## Principles

- Keep the SwiftUI client native, compact, and desktop-first instead of porting the React/Tauri interface.
- Keep SwiftUI as the source of truth for value state, selection, commands, and layout.
- Use AppKit only for narrow platform gaps such as cursor behavior, local scroll tuning, text-system control, panels, responder-chain access, or window behavior SwiftUI cannot express cleanly.
- Prefer stable split layouts and explicit selection over top-level root swapping.
- Keep build, run, and visual inspection reproducible through one project-local command.
- Preserve behavior during structural refactors; make visual changes only after file boundaries are clear.

## Current Shape

- SwiftPM executable package: `swiftui/ProxymanClient`.
- Main app entry: `ProxymanClientApp`.
- Root UI lives in `ContentView.swift`; capture, rules, system proxy, certificates, and shared workspace chrome now live in dedicated view files under `Views/`.
- App state, mock client, socket client boundary, and event application logic are concentrated in `Models.swift` and `UnixSocketCoreClient.swift`.
- Shared support modifiers and styles live under `Support/`, including cursor affordances and `ProxymanButtonStyle`.
- Existing smoke path is `scripts/run-swiftui-minimal-test.sh`, but it still references the legacy `src-tauri` sidecar path.

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

## Target File Shape

- `App/`: app entry, scene setup, command ownership, app delegate if needed.
- `Views/`: root layout and feature surfaces.
- `Views/Shared/`: reusable chrome, rows, badges, preview sections, status components.
- `Models/`: pure value models, identifiers, payload-independent UI state.
- `Stores/`: observable app state and feature-specific state coordinators.
- `Services/`: sidecar launch, Unix socket client, command/event clients.
- `Support/`: AppKit bridges, formatters, cursor helpers, lightweight view modifiers.

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
- [ ] Move pure models and enums out of the observable app store file.
- [ ] Split the observable app state into focused stores or coordinators where the dependency boundary is clear.
- [ ] Move sidecar launch and Unix socket transport into `Services/`.
- [ ] Replace the ad hoc SwiftUI smoke script with `script/build_and_run.sh`.
- [ ] Add `.codex/environments/environment.toml` once the run script is stable.
- [ ] Add narrow SwiftPM tests for parsing, event application, and store behavior before changing those code paths.
- [ ] Run a visual smoke pass after structural changes compile.

## AppKit Use Rules

- Wrap AppKit behavior in small `NSViewRepresentable`, modifier, or service types.
- Do not pass `NSView`, `NSWindow`, or `NSCursor` through feature view hierarchies.
- Keep cursor and text-system bridges in `Support/`.
- Keep scroll-view tuning local to the specific SwiftUI surface that needs it.
- Re-check AppKit bridges after moving views, because SwiftUI may recreate representables and modifiers.

## Build And Verification

- Use `swift build --package-path swiftui/ProxymanClient` after each structural slice.
- Use the sidecar smoke path only as a temporary bridge until the sidecar crate is separated from legacy Tauri paths.
- Once `script/build_and_run.sh` exists, prefer it for build, launch, logs, and visual smoke checks.
- Do not run broad formatters as part of these refactors.
