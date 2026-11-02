# Visual direction

The web client takes the look Google AI Studio designed for it: its palette, radius, type,
surfaces and screens (approved by Mohammed; migration in the
[plan](../plans/2026-11-02-ai-studio-ui-migration.md)).

The existing navigation, page layout and Core behavior stay authoritative. The look makes
controls, selected items, section headings and real operation states easy to tell apart.
Light mode keeps the same hierarchy. Color never replaces the text or icon of an operation
state.

## Look

- Palette: the design's values for the dark and the light theme, as tokens in
  `web/src/index.css`. The warning tokens stay. A tint the design needs and the palette
  lacks becomes one token, defined in both themes; no colour is written in a component.
- Radius: the design's `--radius` (`.75rem`).
- Type: Plus Jakarta Sans for text and JetBrains Mono for code; Geist (variable) is loaded as the fallback.
- Surfaces: the design's panel, glow, active-pill and grid-backdrop treatments, built on
  the palette tokens so they hold in light mode.
- Motion: bounded; no continuous decorative animation. Reduced motion is respected.

## Ownership and dependencies

`web/src/index.css` owns palette and effects. Shared UI controls consume those tokens;
features own their existing state and use semantic classes. No new HTTP rule, event
surface or composition root. The companion panel follows its own CSS owner and typed
platform/native boundaries in the accepted agents-and-panel design.

Existing Base UI/shadcn, Tailwind, Motion and Lucide cover the controls. The fonts ship
locally from `@fontsource/plus-jakarta-sans` and `@fontsource/jetbrains-mono` (approved by
Mohammed); no font is loaded from a CDN.

## Planned features

A feature the design shows and Core does not yet serve may show, inside the `Planned`
marker, with no invented value: a dash, an empty bar, a disabled control, never a guess or
a sample. No progress percentage, model state, memory metric or runtime capability is
computed in the client; each one shown is Core's answer or sits empty inside `Planned`.
Each `Planned` block's reason names the shared request in `docs/shared/requests/` it waits
for.
