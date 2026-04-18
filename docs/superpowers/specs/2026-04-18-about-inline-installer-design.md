# CCSwitch About Inline Installer Design

## Summary

Replace the modal-based installer entry in `Settings -> About` with an inline installer workspace that is always visible inside the About page.

The inline workspace must let users:

1. detect `node`, `npm`, `pnpm`, `git`, `claude`, `codex`, `gemini`, and `opencode`
2. see each dependency as a card immediately, even while detection is still running
3. install a single dependency directly from its card
4. install all missing/outdated dependencies with one action
5. install an arbitrary selected subset of missing/outdated dependencies
6. review install progress and manual commands without leaving the About page

## Goals

- Remove the extra modal step from the installer workflow
- Keep detection, install actions, manual commands, and progress in one visible area
- Support both card-level and batch install flows
- Preserve current About page styling and motion language
- Reuse the existing backend installer pipeline where possible

## Non-Goals

- Add a new settings tab or onboarding flow
- Rebuild the installer as a separate screen
- Add background install-session persistence
- Expand platform support beyond the current installer service behavior

## Scope

### Dependencies managed inline

- Core dependencies
  - `node`
  - `npm`
  - `pnpm`
  - `git`
- CLI tools
  - `claude`
  - `codex`
  - `gemini`
  - `opencode`

### Removed interaction

- The About page must no longer require clicking an installer-launch card or button to access the install workflow
- The About page must no longer present the installer flow as a dedicated dialog for this feature path

## User Experience

### About Page Layout

The local environment area in `AboutSection` becomes the full installer workspace. It contains:

1. a section header and detection refresh action
2. a batch action bar
3. a `Core Dependencies` card grid
4. a `CLI Tools` card grid
5. an inline manual-commands section
6. an inline install-progress section

The old `Environment Check & Install` launcher card is removed.

### Card Behavior

Each dependency card shows:

- dependency name
- dependency kind
- state badge or success icon
- detected version or loading/unknown text
- detected path when available
- message/reason when present
- card-level action controls

#### Loading

- Cards render immediately from fixed dependency lists
- While detection is running, cards show spinner state instead of waiting for backend results

#### Installed

- Show success state
- No checkbox
- No install button

#### Missing / Outdated with auto install support

- Show a selection checkbox
- Show an install button on the card
- Permit inclusion in batch selected install

#### Missing / Outdated without auto install support

- No batch-selection checkbox
- Show a `Manual Install` action instead of an auto-install button

#### Broken / Manual

- Do not participate in batch selection
- Show the reason text
- Expose `Manual Install` or equivalent jump-to-commands action

### Batch Actions

The top action bar includes:

- `Refresh Detection`
- `Install All Missing Dependencies`
- `Install Selected Dependencies`

Behavior rules:

- `Install All Missing Dependencies` targets all dependencies in `missing` or `outdated`
- `Install Selected Dependencies` targets only selected dependencies that also support auto install
- Selected count may be reflected in the button label
- Install and refresh actions must be disabled while an install run is active

### Special Dependency Rule: npm

`npm` is not installed independently in the current backend model. When `npm` is missing, the installation action must clearly communicate that the real action is installing Node.js, for example `Install Node.js (includes npm)`.

This wording must apply anywhere the user can install `npm`, including card-level actions.

### Manual Commands

- Manual commands are shown inline below the card grids
- The section is collapsed by default unless the user asks for manual installation
- Clicking a card-level manual action expands the section and scrolls to the matching dependency command group

### Progress

- Install progress remains visible inline on the About page
- The panel shows recent install steps in order
- After installation completes, the app automatically re-runs environment detection so the cards refresh in place

## Architecture

### Frontend

Primary file changes:

- Modify `src/components/settings/AboutSection.tsx`
- Modify or replace `src/components/settings/InstallerDependencyCard.tsx` so it can support inline actions and selection state
- Reuse `src/components/settings/InstallerProgressPanel.tsx`
- Stop using `src/components/settings/InstallerCenterDialog.tsx` from the About page path
- Extend `src/lib/api/installer.ts`
- Extend `src/types/installer.ts` if needed for new API inputs

Frontend responsibilities:

- own the selected-dependency state
- map backend detection results into card grids
- render inline install actions and button disable states
- invoke batch install-all and install-selected flows
- invoke single-dependency install by passing a one-item selection
- manage manual-command expansion and target focus
- reset/reload detection after installs

### Backend

Primary file changes:

- Modify `src-tauri/src/commands/installer.rs`
- Modify `src-tauri/src/services/installer/install.rs`
- Modify `src-tauri/src/lib.rs` to register the new command

Backend responsibilities:

- keep existing `install_missing_dependencies` behavior for install-all
- add a new selected-install command that accepts dependency names
- reuse existing planning/ordering logic so selected installs still honor prerequisites such as Node before `pnpm` or `npm`-backed tools
- emit the same install progress events used by the current progress panel

## Command API

Required Tauri commands:

- `detect_installer_environment`
- `install_missing_dependencies`
- `install_selected_dependencies`
- `get_manual_install_commands`

### `install_selected_dependencies`

Input:

- array of `InstallerDependencyName`

Behavior:

- normalize the requested names against current environment state
- ignore already healthy dependencies
- preserve prerequisite ordering
- return the same `InstallerRunResult` shape as install-all

## Error Handling

- If detection fails, keep cards visible and show unknown/error state rather than removing the installer workspace
- If selected install is requested with no valid installable targets, return a no-op result instead of crashing the UI
- Manual-only dependencies must never appear as auto-installable selections

## Testing

Frontend tests must cover:

- About page renders both core and tool cards immediately
- missing auto-installable cards expose checkbox and install action
- manual-only cards expose manual-install action instead of auto-install
- install-all and install-selected button enable/disable rules
- single-card install calls the selected-install API with one dependency
- inline manual command section expands and reveals relevant command content

Backend tests must cover:

- selected install plan preserves Node-first ordering
- selected install ignores already installed dependencies
- selected install can target a single tool
- selected install treats `npm` as Node-backed installation
