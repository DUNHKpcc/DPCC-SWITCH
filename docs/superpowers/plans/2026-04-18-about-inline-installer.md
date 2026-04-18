# About Inline Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the installer workflow into `Settings -> About` so users can detect, select, install, and inspect dependencies inline.

**Architecture:** Extend the installer backend with a selected-dependency install command, then refactor `AboutSection` into the single installer surface that owns card actions, batch selection, manual commands, and progress. Reuse the existing progress panel and dependency metadata where practical, but remove the modal-based interaction from the About-page path.

**Tech Stack:** React, TypeScript, Vitest, Tauri commands, Rust installer service

---

### Task 1: Add the selected-install backend API

**Files:**
- Modify: `src-tauri/src/services/installer/install.rs`
- Modify: `src-tauri/src/commands/installer.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add tests in `src-tauri/src/services/installer/install.rs` that assert:

```rust
#[test]
fn selected_install_plan_puts_node_before_requested_pnpm() {
    let plan = build_selected_install_plan(
        &[InstallerDependencyName::Pnpm],
        &[
            status(InstallerDependencyName::Node, InstallerDependencyKind::Core, InstallerDependencyState::Missing),
            status(InstallerDependencyName::Pnpm, InstallerDependencyKind::Core, InstallerDependencyState::Missing),
        ],
    );

    assert_eq!(plan, vec![InstallerDependencyName::Node, InstallerDependencyName::Pnpm]);
}
```

- [ ] **Step 2: Run the Rust installer tests to verify they fail**

Run: `cargo test installer::install --lib`
Expected: FAIL because `build_selected_install_plan` and selected-install execution do not exist yet.

- [ ] **Step 3: Implement the selected-install planner and command**

Add a planner and executor shaped like:

```rust
pub fn build_selected_install_plan(
    requested: &[InstallerDependencyName],
    dependencies: &[InstallerDependencyStatus],
) -> Vec<InstallerDependencyName> {
    let requested: std::collections::BTreeSet<_> = requested.iter().copied().collect();
    let filtered: Vec<_> = dependencies
        .iter()
        .filter(|dependency| requested.contains(&dependency.name))
        .cloned()
        .collect();
    build_install_plan(&filtered)
}
```

and a Tauri command:

```rust
#[tauri::command]
pub async fn install_selected_dependencies(
    app: AppHandle,
    dependencies: Vec<crate::services::installer::types::InstallerDependencyName>,
) -> Result<crate::services::installer::install::InstallerRunResult, String> {
    crate::services::installer::install::install_selected_dependencies(&app, &dependencies).await
}
```

- [ ] **Step 4: Run the Rust tests again**

Run: `cargo test installer::install --lib`
Expected: PASS

### Task 2: Add the frontend installer API contract

**Files:**
- Modify: `src/lib/api/installer.ts`
- Modify: `src/types/installer.ts` if helper exports are needed by UI state

- [ ] **Step 1: Write the failing UI test expectation**

Extend `src/components/settings/AboutSection.test.tsx` with an expectation like:

```ts
expect(installerApi.installSelected).toHaveBeenCalledWith(["codex"]);
```

for a future card-level install interaction.

- [ ] **Step 2: Run the About-section test to verify it fails**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: FAIL because `installSelected` is not exposed or called.

- [ ] **Step 3: Add the API method**

Expose:

```ts
installSelected(
  dependencies: InstallerDependencyName[],
): Promise<InstallerRunResult> {
  return invoke("install_selected_dependencies", { dependencies });
}
```

- [ ] **Step 4: Re-run the About-section test**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: still FAIL, but now on missing UI behavior rather than missing API surface.

### Task 3: Refactor About page into the inline installer

**Files:**
- Modify: `src/components/settings/AboutSection.tsx`
- Modify: `src/components/settings/InstallerDependencyCard.tsx`
- Reuse: `src/components/settings/InstallerProgressPanel.tsx`

- [ ] **Step 1: Write failing About-section tests for inline installer behavior**

Cover:

```ts
test("shows install actions on missing dependencies inline", async () => {
  render(<AboutSection isPortable={false} />);
  expect(await screen.findByRole("button", { name: /install all missing dependencies/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /install selected dependencies/i })).toBeDisabled();
  expect(screen.getByRole("checkbox", { name: /select codex/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: FAIL because the About page still renders the old launcher card/modal flow.

- [ ] **Step 3: Implement the inline installer state and layout**

In `AboutSection.tsx`, add state similar to:

```ts
const [manualCommands, setManualCommands] = useState<ManualInstallCommandGroup[]>([]);
const [progress, setProgress] = useState<InstallExecutionStep[]>([]);
const [installing, setInstalling] = useState(false);
const [selectedDependencies, setSelectedDependencies] = useState<InstallerDependencyName[]>([]);
const [showManualCommands, setShowManualCommands] = useState(false);
```

and render:

```tsx
<div className="flex flex-wrap gap-2 px-1">
  <Button onClick={() => void loadAllToolVersions()} disabled={isLoadingTools || installing}>...</Button>
  <Button onClick={() => void handleInstallAll()} disabled={!canInstallAll || installing}>...</Button>
  <Button onClick={() => void handleInstallSelected()} disabled={!canInstallSelected || installing}>...</Button>
</div>
```

- [ ] **Step 4: Implement card-level action support**

Update `InstallerDependencyCard.tsx` to accept props for:

```ts
selected?: boolean;
selectable?: boolean;
installLabel?: string;
installing?: boolean;
onToggleSelected?: () => void;
onInstall?: () => void;
onShowManual?: () => void;
```

and render checkbox/install/manual actions inside the card footer.

- [ ] **Step 5: Replace the About-page launcher card**

Remove the `Environment Check & Install` launcher block and `InstallerCenterDialog` usage from `AboutSection.tsx`. Keep the progress panel and manual commands inline below the dependency grids.

- [ ] **Step 6: Run the About-section tests again**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: PASS

### Task 4: Verify manual commands and progress behavior

**Files:**
- Modify: `src/components/settings/AboutSection.test.tsx`
- Optionally trim obsolete `src/components/settings/InstallerCenterDialog.test.tsx` coverage if the dialog is no longer used from About

- [ ] **Step 1: Write tests for manual-command reveal and selected install**

Add tests that:

```ts
await user.click(screen.getByRole("button", { name: /manual install/i }));
expect(await screen.findByText(/package manager or nvm/i)).toBeInTheDocument();
```

and:

```ts
await user.click(screen.getByRole("checkbox", { name: /select codex/i }));
await user.click(screen.getByRole("button", { name: /install selected dependencies/i }));
expect(installerApi.installSelected).toHaveBeenCalledWith(["codex"]);
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: FAIL until inline manual/selection actions are fully wired.

- [ ] **Step 3: Finish the minimal implementation**

Wire:

```ts
const handleInstallSelected = async (dependencies: InstallerDependencyName[]) => {
  setInstalling(true);
  setProgress([]);
  try {
    const result = await installerApi.installSelected(dependencies);
    setProgress(result.steps);
    await loadAllToolVersions();
  } finally {
    setInstalling(false);
  }
};
```

and ensure manual actions set:

```ts
setShowManualCommands(true);
setManualAnchor(dependency.name);
```

- [ ] **Step 4: Re-run the UI tests**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: PASS

### Task 5: Final verification

**Files:**
- Verify: `src/components/settings/AboutSection.tsx`
- Verify: `src/components/settings/InstallerDependencyCard.tsx`
- Verify: `src/lib/api/installer.ts`
- Verify: `src-tauri/src/services/installer/install.rs`

- [ ] **Step 1: Run targeted frontend tests**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: PASS

- [ ] **Step 2: Run targeted Rust installer tests**

Run: `cargo test installer::install --lib`
Expected: PASS

- [ ] **Step 3: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS
