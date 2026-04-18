# Installer Strategy Review

## Scope

Review target:

- `src-tauri/src/services/installer/detect.rs`
- `src-tauri/src/services/installer/install.rs`
- `src/components/settings/AboutSection.tsx`

Focus:

- install support matrix
- dependency planning
- fallback/manual-install strategy
- post-install verification semantics

## Defects

### 1. macOS exposes Node as auto-installable, but the flow is not implemented

- Severity: high
- Evidence:
  - `detect.rs` marks macOS `node` and `npm` as `auto_install_supported = true`
  - `AboutSection.tsx` allows them into the selectable/installable set
  - `install.rs` returns `Node PKG flow must be implemented...` on macOS
- Impact:
  - users can start an install path that is guaranteed to fail
  - `pnpm` / `codex` / `gemini` selected installs can also be blocked because they inject `Node` as a prerequisite

### 2. Linux support matrix is internally inconsistent

- Severity: high
- Evidence:
  - `detect.rs` marks all Linux dependencies as `auto_install_supported = false`
  - `install.rs` already contains Linux install commands for `pnpm`, `claude`, `codex`, `gemini`, and `opencode`
- Impact:
  - the UI disables batch install for Linux even where the backend already supports it
  - strategy and implementation disagree on what the product supports

### 3. Manual fallback commands are incomplete for Node-dependent tools

- Severity: medium
- Evidence:
  - manual commands for `pnpm`, `codex`, and `gemini` all require `npm`
  - planner logic explicitly treats these tools as Node/npm-backed dependencies
- Impact:
  - when Node/npm is missing, the fallback commands for these tools are not executable
  - the manual path does not reliably recover the broken environment

### 4. The strategy claims `outdated` / `broken` handling, but detection never produces those states

- Severity: medium
- Evidence:
  - `install.rs` plans for `Missing | Outdated`
  - UI renders `outdated` and `broken`
  - `detect.rs` currently only derives `installed`, `missing`, and `manual`
- Impact:
  - upgrade and repair flows are mostly declarative, not real
  - users may believe the installer supports remediation that is not actually implemented

### 5. Success is based on command exit code, not verified environment state

- Severity: medium
- Evidence:
  - `execute_install_plan` records `Completed` immediately after the install command exits successfully
  - there is no in-command verify pass even though `Verifying` exists in the progress enum
  - final correctness depends on a later frontend refresh
- Impact:
  - progress can report success before the dependency is actually usable on PATH
  - shell/profile-based installers can appear complete while the environment still resolves as missing

## Recommended Priority

1. Align the support matrix first:
   - stop advertising unsupported macOS Node install
   - decide whether Linux CLI auto-install is supported, then make detect/UI/install consistent
2. Fix Node-dependent fallback strategy:
   - ensure manual commands are executable in no-Node scenarios
3. Add real verification semantics:
   - verify the dependency after install before reporting final success
4. Either implement or remove `outdated` / `broken` remediation claims

## Conclusion

The current installer strategy is not yet internally consistent. The main blockers are the false-positive support declaration on macOS Node installation and the false-negative support declaration for Linux CLI installation.
