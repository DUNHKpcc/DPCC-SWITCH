# Node Auto Install Design

## Summary

Extend the existing installer workflow so `Node.js` can be auto-installed on `Windows` and `macOS` without introducing a new UI surface.

The implementation must keep the current About page installer workspace and reuse the existing installer service, progress panel, dependency cards, and manual-command fallback area.

The first version intentionally excludes Linux `Node` auto-install.

## Goals

- Add a real `Node.js` auto-install path for `Windows` and `macOS`
- Keep installer behavior internally consistent across detect, plan, install, verify, and UI
- Support weak-network `macOS` users by bootstrapping `Homebrew` from a domestic mirror when `brew` is missing
- Preserve the current About page installer layout and styling with minimal UI churn
- Verify `node` and `npm` after installation before reporting success

## Non-Goals

- Add Linux `Node` auto-install in this project
- Introduce a new installer screen, modal, or onboarding flow
- Add official Node `pkg` / `msi` download-and-run fallback in v1
- Implement background privilege escalation beyond the platform-native flows already required by the chosen package manager path
- Generalize the installer into a full workflow engine for unrelated dependencies

## Scope

### Platforms

- `Windows`
  - auto-install `Node.js` only when `winget` is available
  - require the app to already be running with administrator privileges before starting install
- `macOS`
  - auto-install `Node.js` through `Homebrew`
  - if `brew` is missing, bootstrap `Homebrew` from a domestic mirror first
  - allow the bootstrap/install flow to trigger native system authorization when required
- `Linux`
  - keep `Node.js` as manual-install only

### UI surfaces

- `src/components/settings/AboutSection.tsx`
- existing installer progress panel
- existing manual commands section
- existing dependency card presentation

No new installer surface is introduced.

## Product Behavior

### Support Matrix

`Node` and `npm` auto-install support must be derived from real executable platform capability, not from a static optimistic flag.

- `Windows`
  - auto-install supported only when:
    - `winget` is available
    - the app process is running with administrator privileges
- `macOS`
  - auto-install supported when:
    - `brew` is already available, or
    - the app can attempt `Homebrew` bootstrap through the domestic mirror flow
- `Linux`
  - auto-install unsupported

If any required precondition is missing, the dependency must present as `manual` or a failed run with a concrete reason, not as auto-installable.

### Windows Flow

1. Detect whether `winget` exists on PATH
2. Detect whether the current app process is elevated
3. If `winget` is missing:
   - do not offer `Node` as auto-installable
   - expose manual commands and message text that explain the fallback
4. If the app is not elevated:
   - do not run install
   - tell the user to reopen the app as administrator
5. If preflight passes:
   - run `winget install OpenJS.NodeJS.LTS -e --source winget`
6. Re-detect `node` and `npm`
7. Report success only when both are available on PATH

### macOS Flow

1. Detect whether `brew` exists on PATH
2. If `brew` exists:
   - run `brew install node`
3. If `brew` does not exist:
   - bootstrap `Homebrew` through the domestic mirror source
   - allow the process to request native system authorization when the bootstrap requires it
   - if bootstrap succeeds, run `brew install node`
4. Re-detect `node` and `npm`
5. Report success only when both are available on PATH
6. If the user rejects authorization, or the mirror bootstrap fails:
   - stop the flow
   - expose manual commands for mirrored `Homebrew` install plus `brew install node`

### Linux Flow

- `Node` remains manual-only
- no auto-install attempt should be scheduled

## Architecture

### Backend

Primary files:

- Modify `src-tauri/src/services/installer/detect.rs`
- Modify `src-tauri/src/services/installer/install.rs`
- Modify `src-tauri/src/services/installer/types.rs` only if current types cannot carry the new capability or message semantics

Backend responsibilities:

- detect package-manager and privilege prerequisites for `Node`
- derive whether `Node` is actually auto-installable on the current machine
- plan `Node` installation only when its full prerequisite chain is executable
- execute platform-specific install flows
- verify `node` and `npm` after install before reporting completion
- emit detailed progress messages through the existing progress event stream
- return actionable failure/manual messages that the UI can render directly

### Detection Model

Detection should stay centered on the current installer environment response, but it needs platform-aware capability checks in addition to binary presence checks.

Recommended helpers:

- `detect_winget_available()`
- `detect_windows_elevation()`
- `detect_homebrew_available()`
- `node_auto_install_capability_for_platform()`

`detect.rs` should continue to produce dependency-level status objects, but those statuses should reflect real machine capability:

- `missing` only when the dependency can actually be auto-installed
- `manual` when the machine is missing a required package manager or prerequisite path
- `broken` when install/verify leaves `node` and `npm` in a partial state

### Installation Model

Keep the current installer orchestration entrypoints, but isolate Node-specific platform execution into focused helpers rather than expanding a single large `match` body.

Recommended helpers:

- `install_node_on_windows()`
- `install_node_on_macos()`
- `ensure_homebrew_with_domestic_mirror()`
- `verify_node_runtime()`

The existing execution pipeline should remain:

1. queue/install progress emission
2. dependency install execution
3. verify pass
4. normalize result

The Node branch should plug into that same pipeline.

### Verification Semantics

Success must require a fresh verify pass after install.

- verify `node --version`
- verify `npm --version`
- verify both resolve on PATH

Result rules:

- both available: `completed`
- only one available: `broken`
- neither available: `failed` or `manual`, depending on the failure reason

Frontend refresh remains useful for visual sync, but it must no longer be the source of truth for install correctness.

## UI Design

### Reuse Principles

The UI must stay visually aligned with the current About page installer design.

- reuse existing dependency cards
- reuse existing batch action bar
- reuse existing progress panel
- reuse existing manual commands section
- reuse current card spacing, typography, badge language, and motion behavior

Do not add:

- a new modal
- a new dedicated Node installer screen
- special-case visual chrome that breaks the current installer look

### Card Behavior

`Node` should continue to appear in the existing dependency grid.

The card message should become more specific based on platform state:

- `Windows`, no `winget`: explain that `winget` is required for auto-install
- `Windows`, not elevated: explain that the app must be reopened as administrator
- `macOS`, no `brew`: explain that `Homebrew` will be installed first from the domestic mirror
- `macOS`, bootstrap/authorization failure: explain the fallback manual path

Selection and batch-install affordances should continue following the existing rules:

- only show checkbox when auto-install is really executable
- keep manual-only states out of selectable install sets

### Progress Panel

Reuse the existing panel and stage badges.

Prefer richer stage messages over inventing many new enum stages.

Examples:

- `Checking winget availability...`
- `Checking administrator privileges...`
- `Installing Node.js with winget...`
- `Checking Homebrew availability...`
- `Installing Homebrew from domestic mirror...`
- `Installing Node.js with Homebrew...`
- `Verifying node and npm on PATH...`

### Manual Commands

Manual fallback commands must stay in the current inline manual-command section.

Platform-specific expectations:

- `Windows`
  - explain that the app should be reopened as administrator when privilege is the blocker
  - include `winget` or official Node download fallback guidance when `winget` is unavailable
- `macOS`
  - if `brew` exists, provide `brew install node`
  - if `brew` does not exist, provide domestic-mirror `Homebrew` bootstrap commands first, then `brew install node`

## Error Handling

- If `winget` is missing, treat Windows Node install as manual-only
- If the Windows process is not elevated, fail fast with a clear retry instruction
- If `Homebrew` bootstrap on macOS requires authorization and the user rejects it, stop and expose manual commands
- If `Homebrew` bootstrap from the domestic mirror fails, stop and expose manual commands
- If install command exits successfully but verify fails, do not report success
- If only one of `node` / `npm` becomes available, classify the runtime as `broken`

## Testing

### Rust tests

Add or update tests covering:

- Node support matrix for `Windows`, `macOS`, and `Linux`
- Windows Node auto-install disabled when `winget` is missing
- Windows Node install blocked when not elevated
- macOS Node flow choosing existing `brew`
- macOS Node flow entering mirror bootstrap when `brew` is missing
- verify pass requiring both `node` and `npm`
- partial verify results producing `broken`
- plan filtering so Node-dependent tools only auto-install when Node runtime installation is actually executable

### Frontend tests

Add or update tests covering:

- Node card messaging for Windows no-`winget` and non-admin scenarios
- Node card messaging for macOS `brew` bootstrap scenario
- selection checkbox visibility staying aligned with actual auto-install support
- progress panel rendering Node-specific progress text
- manual-command section showing platform-appropriate fallback commands

### Manual validation

Run real smoke checks on:

- Windows with `winget` available and app elevated
- Windows without `winget`
- Windows without elevation
- macOS with `brew` already installed
- macOS without `brew`, including the domestic-mirror bootstrap path
- macOS bootstrap authorization rejection path

## Open Decisions Resolved

- First version supports `Windows + macOS` only
- `Windows` requires the user to reopen the app as administrator instead of hidden elevation attempts
- `macOS` may request native system authorization during `Homebrew` bootstrap
- `Homebrew` bootstrap should use a domestic mirror when `brew` is missing
- UI stays inside the existing About page installer workspace and should reuse current components wherever possible
