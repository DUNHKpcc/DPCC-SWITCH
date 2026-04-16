# CCSwitch Installer Center Design

## Summary

Add a new dependency detection and installation entry under `Settings -> About` in `cc-switch`.
The new entry opens a dedicated installer panel that brings ClaudeHub's core workflow into CCSwitch:

1. detect local environment
2. show dependency status cards
3. install missing or outdated dependencies
4. stream install progress
5. re-check environment after install

The implementation must preserve `cc-switch`'s existing product language and component system. We are migrating capability and flow, not ClaudeHub's standalone visual shell.

## Goals

- Add a native-feeling installer workflow inside `cc-switch`
- Detect and manage both foundational dependencies and supported CLI tools
- Support automatic install on Windows and macOS where supported
- Support detection plus manual install guidance on Linux
- Keep backend installation logic isolated from existing settings and proxy flows

## Non-Goals

- Rebuild ClaudeHub's renderer UI or inline-style system
- Add a new top-level settings tab or first-run onboarding in this iteration
- Persist cross-window install session recovery in v1
- Support unattended Linux auto-install in v1

## Scope

The installer panel manages:

- Core dependencies
  - `node`
  - `npm`
  - `git`
- Tool dependencies
  - `claude`
  - `codex`
  - `gemini`
  - `opencode`

The installer entry lives inside the existing About page.

## User Experience

### Entry Point

`Settings -> About` gains a new card-style entry named something equivalent to `Environment Check & Install`.

This entry should:

- explain that it can detect and install required local CLI dependencies
- open a dedicated dialog or full-panel modal
- not replace existing About page update/version content

### Installer Panel Layout

The installer panel contains four sections:

1. Environment summary
2. Dependency cards
3. Action bar
4. Progress panel

### Environment Summary

The summary shows:

- current platform
- whether automatic install is supported on this platform
- last detection time
- overall readiness summary

### Dependency Cards

Dependencies are grouped into:

- Core dependencies
- CLI tools

Each card shows:

- dependency name
- kind
- current state
- detected version if available
- detected path if available
- short explanation or remediation message
- whether auto-install is supported for this dependency on the current platform

### Action Bar

The action bar includes:

- `Re-check Environment`
- `Install Missing Dependencies`
- `View Manual Commands`

Behavior rules:

- `Install Missing Dependencies` installs only `missing` or `outdated` items
- already healthy dependencies are skipped
- on Linux, auto-install is not offered; manual guidance is shown instead
- install and refresh actions must disable conflicting actions while running

### Progress Panel

The progress panel shows recent install events in order.

Each event includes:

- dependency name
- stage
- user-facing message

The panel remains visible after completion so the user can inspect what happened.

## Platform Behavior

### Windows

Support auto detection and auto install.

Expected install approach:

- Node.js: installer package flow
- Git: `winget` first if appropriate, with official installer fallback
- Tool CLIs: shell-based install commands once prerequisites exist

### macOS

Support auto detection and auto install for Node.js and supported CLI tools.

Git behavior must be conservative:

- detect and report normally
- if Git is missing, prefer a manual guidance path instead of pretending full automation is reliable
- the UI should clearly distinguish `manual` vs `failed`

### Linux

Support detection only in v1.

The panel must:

- show dependency state
- provide manual commands
- avoid implying that one-click install is available

## Architecture

### Frontend

New frontend modules:

- `src/components/settings/InstallerCenterDialog.tsx`
- `src/components/settings/InstallerDependencyCard.tsx`
- `src/components/settings/InstallerActions.tsx`
- `src/components/settings/InstallerProgressPanel.tsx`
- `src/lib/api/installer.ts`
- `src/types/installer.ts`

Existing frontend modules to modify:

- `src/components/settings/AboutSection.tsx`

Frontend responsibilities:

- open and close installer panel
- request environment detection from Tauri
- start install flow
- render grouped dependency states
- render progress updates
- re-run detection after install completion
- show manual commands returned by backend

### Frontend Visual Rules

The installer UI must follow existing `cc-switch` patterns:

- use current `ui` primitives such as `Button`, `Dialog`, `Badge`, and existing card styling patterns
- keep spacing, typography, radius, and motion aligned with current settings pages
- do not import ClaudeHub's custom inline visual system
- use semantic status colors already common in the app
- keep copy tone consistent with `cc-switch` system settings, not a standalone installer app

### Backend

Create a dedicated installer subsystem under `src-tauri`.

Suggested structure:

- `src-tauri/src/commands/installer.rs`
- `src-tauri/src/services/installer/mod.rs`
- `src-tauri/src/services/installer/types.rs`
- `src-tauri/src/services/installer/detect.rs`
- `src-tauri/src/services/installer/install.rs`

Existing backend files to modify:

- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs` or the equivalent Tauri command registration path

Backend responsibilities:

- detect dependency version/path/state
- normalize platform-specific output into stable frontend models
- generate install plans
- execute installers and CLI install commands
- emit install progress events
- generate manual install commands per platform
- re-verify each dependency after installation

### Tauri Command API

Add the following commands:

- `detect_installer_environment`
- `install_missing_dependencies`
- `get_manual_install_commands`

Optional future command, not required for v1:

- `get_installer_progress_state`

### `detect_installer_environment`

Returns:

- platform metadata
- whether automatic install is supported
- dependency list
- last checked timestamp
- summary readiness information

### `install_missing_dependencies`

Behavior:

- detect current dependency states
- compute install plan from current state
- run install steps in order
- emit progress events to frontend
- return per-dependency result summary
- trigger frontend re-check after completion

### `get_manual_install_commands`

Returns platform-specific command groups for:

- Node.js
- Git
- Claude Code
- Codex
- Gemini CLI
- OpenCode

The frontend must display these as backend-authored guidance, not hardcoded strings.

### Shared Data Model

The backend and frontend must align on a stable installer model.

### `InstallerDependencyStatus`

Fields:

- `name`
- `kind`
- `state`
- `version`
- `path`
- `message`
- `auto_install_supported`

`kind` values:

- `core`
- `tool`

`state` values:

- `installed`
- `missing`
- `outdated`
- `broken`
- `manual`

### `InstallerEnvironment`

Fields:

- `platform`
- `auto_install_supported`
- `dependencies`
- `last_checked_at`
- `ready_count`
- `total_count`

### `InstallProgressEvent`

Fields:

- `dependency`
- `stage`
- `message`

`stage` values:

- `queued`
- `downloading`
- `installing`
- `verifying`
- `completed`
- `failed`
- `manual`

### `InstallResult`

Fields:

- `steps`
- `completed_dependencies`
- `failed_dependencies`
- `manual_dependencies`
- `status_message`

## Detection Rules

Detection must identify version, path, and operational status where possible.

Rules:

- `node` and `npm` are linked prerequisites but reported as separate dependencies
- `npm` should be treated as satisfied only when a valid npm binary is detectable
- tool CLIs should be detected with version commands that are stable enough for parsing
- command-not-found conditions map to `missing`
- known broken states or verification mismatches map to `broken`
- older but still present versions map to `outdated` when minimum version rules exist

## Install Planning Rules

Install planning must be deterministic.

Required order:

1. `node`
2. `git`
3. `claude`
4. `codex`
5. `gemini`
6. `opencode`

Special rule:

- if either `node` or `npm` is missing or outdated, install the `node` dependency first because npm is bundled through that path

General rules:

- skip already healthy dependencies
- continue with later independent tool installs when one tool install fails
- stop only when a prerequisite failure makes later steps invalid
- after each install, immediately re-verify that dependency

## Install Strategy

### Node.js

Reuse ClaudeHub's proven strategy where possible:

- Windows: MSI installer flow
- macOS: PKG installer flow
- support official download source and mirror fallback where appropriate
- re-check `node` and `npm` after install

### Git

Windows:

- attempt supported automated flow first
- use fallback installer strategy if primary path fails

macOS:

- detect normally
- if missing, return `manual` guidance instead of unreliable automation

Linux:

- manual only in v1

### Claude Code

Install using the official shell flow already validated in ClaudeHub, adapted into Rust-side command execution.

### Codex

Install using npm-based global install once Node/npm are available.

### Gemini CLI

Install using npm-based global install once Node/npm are available.

### OpenCode

Install using its official shell-based install flow where appropriate on supported platforms.

## Error Handling

Normalize backend errors into actionable user messages.

Important categories:

- download timeout
- permission denied
- command not found
- verification failed after install
- unsupported platform
- manual-only dependency on current platform

UI behavior:

- show failed dependency clearly
- keep the progress log visible
- preserve completed items even if later steps fail
- allow the user to re-run detection without restarting the app

## Integration Notes

The existing About page already exposes tool version checks. The new installer center must not duplicate that section's implementation details directly.

Recommended boundary:

- About page keeps lightweight version info
- Installer center owns dependency diagnostics and install flows

If practical, version detection helpers may later be shared, but v1 should prioritize clear boundaries over premature reuse.

## Testing Strategy

### Frontend

Add unit tests for:

- dependency grouping and summary rendering
- action availability rules by platform and install state
- progress event rendering
- Linux manual-only behavior

### Backend

Add Rust tests for:

- dependency state normalization
- install plan generation
- manual command generation by platform
- stage/event mapping
- prerequisite handling when `node` or `npm` is missing

### Manual Verification

Manual verification should cover:

- Windows with missing Node/Git/tools
- macOS with missing Claude tool and missing Git
- Linux detection plus manual command display
- install success path
- partial failure path
- post-install re-check path

## Risks

- platform-specific installer behavior differs more in Tauri/Rust than in Electron/Node
- Git automation on macOS is not as reliable as Node/pkg install
- npm global install destinations may vary by user environment
- shell execution and PATH refresh behavior may differ immediately after installation

## Recommended v1 Delivery Shape

Deliver in one focused feature branch with these priorities:

1. backend types, detection, and manual commands
2. frontend dialog and dependency cards
3. install plan execution and progress events
4. platform-specific polishing and verification

## Acceptance Criteria

- About page contains a new installer entry
- opening the installer panel triggers environment detection
- dependency cards render `node`, `npm`, `git`, `claude`, `codex`, `gemini`, and `opencode`
- Windows and macOS provide auto-install where supported
- Linux provides detection and manual guidance only
- install progress streams into the UI
- install completion triggers a fresh environment re-check
- the UI matches `cc-switch` design language rather than ClaudeHub's standalone visual style
