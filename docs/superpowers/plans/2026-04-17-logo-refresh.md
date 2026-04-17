# DPCC-SWITCH Logo Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the visible app logo and packaged application icons with the provided DPCC-SWITCH source image while keeping macOS tray icons unchanged.

**Architecture:** Keep all existing icon entry points and regenerate assets in place so no packaging config changes are needed. The renderer continues to import `src/assets/icons/app-icon.png`, while Tauri packaging continues to read `src-tauri/icons/*`.

**Tech Stack:** Tauri, Vite, macOS icon tooling, shell asset generation

---

### Task 1: Lock Scope And Inputs

**Files:**
- Create: `docs/superpowers/specs/2026-04-17-logo-refresh-design.md`
- Modify: `docs/superpowers/plans/2026-04-17-logo-refresh.md`

- [ ] **Step 1: Confirm the current icon entry points**

Run: `sed -n '1,120p' src/components/settings/AboutSection.tsx && sed -n '1,120p' src-tauri/tauri.conf.json`
Expected: the renderer uses `src/assets/icons/app-icon.png` and Tauri bundle icons point at `src-tauri/icons/*`

- [ ] **Step 2: Confirm the source image metadata**

Run: `file '/Users/dpccskisw/Downloads/Merged Logo.png'`
Expected: a square PNG with transparency that is suitable for icon generation

### Task 2: Replace Renderer And Bundle Icons

**Files:**
- Modify: `src/assets/icons/app-icon.png`
- Modify: `src-tauri/icons/32x32.png`
- Modify: `src-tauri/icons/128x128.png`
- Modify: `src-tauri/icons/128x128@2x.png`
- Modify: `src-tauri/icons/icon.icns`
- Modify: `src-tauri/icons/icon.ico`
- Modify: `src-tauri/icons/icon.png`
- Modify: `src-tauri/icons/Square*.png`
- Modify: `src-tauri/icons/StoreLogo.png`
- Modify: `src-tauri/icons/android/**/*`
- Modify: `src-tauri/icons/ios/*`
- Preserve: `src-tauri/icons/tray/macos/*`

- [ ] **Step 1: Replace the renderer logo**

Run: `cp '/Users/dpccskisw/Downloads/Merged Logo.png' src/assets/icons/app-icon.png`
Expected: `src/assets/icons/app-icon.png` now matches the provided source image

- [ ] **Step 2: Regenerate the Tauri icon set in place**

Run: `pnpm tauri icon '/Users/dpccskisw/Downloads/Merged Logo.png' -o src-tauri/icons`
Expected: Tauri rewrites the standard app icon files under `src-tauri/icons/`

- [ ] **Step 3: Restore the macOS tray template icons if generation touched them**

Run: `git diff --name-only -- src-tauri/icons/tray/macos`
Expected: no output; if any file appears, revert only those tray icon paths to keep them unchanged

### Task 3: Verify Asset Wiring

**Files:**
- Verify: `src/assets/icons/app-icon.png`
- Verify: `src-tauri/icons/*`

- [ ] **Step 1: Confirm the expected files now resolve to the new source**

Run: `file src/assets/icons/app-icon.png src-tauri/icons/32x32.png src-tauri/icons/128x128.png src-tauri/icons/icon.icns src-tauri/icons/icon.ico`
Expected: valid image/icon metadata for each path

- [ ] **Step 2: Run the renderer build**

Run: `pnpm run build:renderer`
Expected: exit code 0 and a populated `dist/` without asset resolution errors

- [ ] **Step 3: Review the final changed file list**

Run: `git status --short`
Expected: icon asset changes plus the plan/spec documents, with no unintended tray icon diffs
