# Black & White Theme Design

**Date:** 2026-04-18

## Goal

Convert the application UI from its current blue-accented theme to a black-and-white visual system across the full app, while preserving:

- original icon colors
- semantic success / warning / error colors
- existing layout, spacing, and interaction behavior unless color is the affected dimension

## Scope

This change applies to the full application UI, not only the settings/about views.

Included:

- global theme tokens in `src/index.css`
- shared UI primitives such as buttons, tabs, inputs, textareas, badges, focus states
- reusable glass/active styles that currently rely on blue
- high-visibility feature components that hardcode `blue-*` utility classes

Excluded:

- icon asset colors and multicolor brand/provider icons
- success, warning, destructive, and other semantic state colors used to communicate status
- provider/theme config data persisted in backend structures where `primary` means business logic, not visual color

## Visual Direction

The UI should read as monochrome:

- light mode: white / off-white surfaces, black / charcoal text, gray borders, black active accents
- dark mode: near-black surfaces, white text, graphite borders, white/gray active accents

Accent behavior should feel neutral rather than branded:

- active buttons/tabs use black in light mode and white/light gray in dark mode
- focus rings use neutral ring colors, not blue
- selected/active glass states use grayscale translucent fills and borders

## Implementation Strategy

### 1. Global tokens first

Update the base CSS variables in `src/index.css` so that `--primary`, `--ring`, and related shared surfaces shift from blue to grayscale values. This ensures components that already consume theme tokens move with minimal code churn.

### 2. Primitive components second

Update shared primitives that still hardcode blue classes:

- `Button`
- `Tabs`
- `Input`
- `Textarea`
- any other shared controls discovered during targeted grep

The goal is to remove blue from the default interaction language of the app.

### 3. Hardcoded blue hotspots last

Search for `bg-blue-*`, `text-blue-*`, `border-blue-*`, `ring-blue-*`, and similar usages in app components. Replace only those that represent general theme styling. Skip:

- icons
- semantic status messaging where blue is not a theme color but a purposeful informational cue, unless it is clearly part of the general UI chrome

When in doubt, prefer neutral grays for informational panels so the app remains visually consistent.

## Testing Strategy

Verification should combine:

- focused unit tests for shared components where practical
- targeted regression checks for components already covered by tests
- a final grep pass to identify leftover hardcoded blue classes in shipped UI code

Success criteria:

- default primary interactions no longer render blue styling
- top-level surfaces and shared controls read as monochrome
- semantic success/warning/error styling remains intact
- icons remain colored

## Risks

### Missed hardcoded blue utilities

There are many scattered `blue-*` classes. A token-only pass will not be enough. Mitigation: perform a targeted grep after primitive updates and clean remaining app-facing hotspots.

### Over-correcting semantic colors

Some blue panels may represent informational meaning rather than branding. Mitigation: keep explicit success/warning/error untouched, and convert only theme-chrome/info surfaces that should align with the new monochrome direction.

### Dark mode contrast regressions

Black-and-white themes can lose affordance in dark mode if borders and active states are too subtle. Mitigation: keep enough contrast between `background`, `card`, `border`, `accent`, and `primary`.
