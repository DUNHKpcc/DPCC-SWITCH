# Black & White Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the app-wide blue-accented UI into a black-and-white theme while preserving icon colors and semantic success/warning/error states.

**Architecture:** Start from global theme tokens and shared primitives so most of the UI shifts automatically, then clean up remaining hardcoded blue utility classes in high-visibility feature surfaces. Verification combines focused tests and a residual grep pass for leftover blue classes.

**Tech Stack:** React, Tailwind CSS, shared UI primitives, Vitest

---

### Task 1: Shift Global Theme Tokens To Monochrome

**Files:**
- Modify: `src/index.css`

- [ ] **Step 1: Update light and dark primary/ring tokens to grayscale**

Change `--primary`, `--primary-foreground`, and `--ring` in both `:root` and `.dark` from blue-based values to neutral grayscale values while keeping background/card/border contrast intact.

- [ ] **Step 2: Update grayscale glass active helpers**

Replace the blue-tinted `.glass-card-active` styles with grayscale translucent backgrounds and borders in `src/index.css`.

- [ ] **Step 3: Update global focus outline color**

Replace the global `outline-blue-500` focus-visible styling with a neutral black/white-aligned outline in `src/index.css`.

- [ ] **Step 4: Verify no syntax regressions in CSS**

Run: `pnpm test:unit -- src/components/settings/AboutSection.test.tsx`
Expected: PASS

### Task 2: Convert Shared Primitives Off Hardcoded Blue

**Files:**
- Modify: `src/components/ui/button.tsx`
- Modify: `src/components/ui/tabs.tsx`
- Modify: `src/components/ui/input.tsx`
- Modify: `src/components/ui/textarea.tsx`

- [ ] **Step 1: Replace primary button blue classes with monochrome classes**

Change default/link button variants in `src/components/ui/button.tsx` so primary actions use black/white styling and links use neutral foreground emphasis instead of blue.

- [ ] **Step 2: Replace tabs active blue classes with monochrome classes**

Change active tab background/text styling in `src/components/ui/tabs.tsx` to black/white equivalents.

- [ ] **Step 3: Replace input and textarea blue focus rings with neutral rings**

Change hardcoded `ring-blue-*` usage in `src/components/ui/input.tsx` and `src/components/ui/textarea.tsx` to grayscale focus rings.

- [ ] **Step 4: Verify shared primitives still satisfy existing tests**

Run: `pnpm test:unit -- tests/components/SettingsDialog.test.tsx`
Expected: PASS

### Task 3: Clean High-Visibility Hardcoded Blue Hotspots

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/config/appConfig.tsx`
- Modify: `src/components/deeplink/SkillConfirmation.tsx`
- Modify: `src/components/DeepLinkImportDialog.tsx`
- Modify: `src/components/FirstRunNoticeDialog.tsx`
- Modify: `src/components/ConfirmDialog.tsx`
- Modify: `src/components/providers/forms/ProviderPresetSelector.tsx`
- Modify: `src/components/providers/forms/GeminiCommonConfigModal.tsx`
- Modify: `src/components/providers/forms/CodexCommonConfigModal.tsx`
- Modify: `src/components/providers/forms/CommonConfigEditor.tsx`
- Modify: `src/components/providers/forms/GeminiFormFields.tsx`
- Modify: `src/components/providers/forms/shared/ApiKeySection.tsx`
- Modify: `src/components/providers/forms/OpenClawFormFields.tsx`

- [ ] **Step 1: Convert general blue-themed info surfaces to grayscale**

Replace blue background/text/border utility classes in informational cards, notices, and helper panels with grayscale equivalents, but keep explicit success/warning/error surfaces untouched.

- [ ] **Step 2: Convert selection and active chrome to grayscale**

Replace blue-selected states in provider preset selectors and similar general UI controls with monochrome selected states.

- [ ] **Step 3: Convert incidental blue links/chrome to grayscale**

Replace non-semantic blue text/link/chip styles in app chrome with neutral black/gray equivalents.

- [ ] **Step 4: Verify focused About/Settings coverage still passes**

Run: `pnpm test:unit -- src/components/settings/AboutSection.test.tsx`
Expected: PASS

### Task 4: Residual Audit And Verification

**Files:**
- Inspect only

- [ ] **Step 1: Audit remaining blue classes**

Run: `rg -n "blue-|outline-blue|ring-blue" src -g '!**/*.test.*'`
Expected: only intentional icon/semantic leftovers remain, or a short actionable list for cleanup

- [ ] **Step 2: Run targeted regression suite**

Run: `pnpm test:unit -- src/components/settings/AboutSection.test.tsx`
Expected: PASS

- [ ] **Step 3: Run targeted settings dialog regression suite**

Run: `pnpm test:unit -- tests/components/SettingsDialog.test.tsx`
Expected: PASS
