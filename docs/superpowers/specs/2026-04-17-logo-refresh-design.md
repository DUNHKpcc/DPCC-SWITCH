# DPCC-SWITCH Logo Refresh Design

## Summary

Replace the visible DPCC-SWITCH app logo and bundled application icons with the provided source image `/Users/dpccskisw/Downloads/Merged Logo.png`.

The change applies to:

- the renderer-visible app logo used in the About page
- the Tauri application icon set used for macOS app bundles, DMG packaging, Windows packaging, and other generated platform assets

The change does not apply to:

- the macOS tray template icons in `src-tauri/icons/tray/macos/`
- deep-link protocol names, bundle identifiers, or other non-visual branding

## Goals

- Make the in-app logo match the user-provided DPCC-SWITCH mark
- Make packaged app icons match the same source mark
- Keep the existing packaging configuration unchanged by regenerating files at the current icon paths

## Non-Goals

- Redesign the logo
- Rebuild the macOS tray icon template set
- Change unrelated app branding or update mechanics

## Implementation Notes

- Use the provided PNG as the single source asset
- Replace `src/assets/icons/app-icon.png` for renderer usage
- Regenerate `src-tauri/icons/*` using Tauri's icon generation workflow so all required icon sizes stay aligned
- Preserve `src-tauri/icons/tray/macos/*` exactly as-is

## Verification

- Confirm the expected icon files were updated on disk
- Run a renderer build to ensure the app still resolves the in-app logo asset correctly
