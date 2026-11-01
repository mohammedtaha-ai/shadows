# Neon clarity

Approved direction: Mohammed chose neon on 2026-11-01, asked for implementation while
away, and requested the remaining agent-panel tasks in the same run. On 2026-11-02 he
chose the design Google AI Studio built for the web client over that neon hierarchy and
approved its migration ([plan](../plans/2026-11-02-ai-studio-ui-migration.md)). This
spec owns the visual direction under both decisions; the second one governs.

The existing navigation, page layout and Core behavior stay authoritative. The visual
system makes controls, selected items, section headings and real operation states easier
to distinguish. Light mode retains equivalent hierarchy. Color never replaces the text or
icon of an operation state.

## Look

AI Studio's palette, radius, type and surfaces replace the neon hierarchy (dark violet
surfaces, luminous edges, glow and sheen) that F-68 built:

- Palette: the export's values for the dark and the light theme, as tokens in
  `web/src/index.css`. The warning tokens stay. A tint the design needs and the palette
  lacks becomes one token, defined in both themes; no colour is written in a component.
- Radius: the export's `--radius` (`.75rem`).
- Type: Plus Jakarta Sans for text and JetBrains Mono for code, as the export uses them.
- Surfaces: the export's panel, glow, active-pill and grid-backdrop treatments, rebuilt on
  the palette tokens so they hold in light mode.

## Approaches

- Quiet premium: subtle purple, little glow. Considered; Mohammed chose the stronger neon.
- Neon clarity: luminous primary/selected controls and layered surfaces, clear text and
  bounded motion. Selected 2026-11-01, built in F-68, replaced 2026-11-02.
- Full-screen effects: continuous glow animations and decoration. Not needed for the
  requested clarity; it would obscure content and increase rendering work.
- AI Studio design: its palette, radius, type and surfaces, and its screens. Selected
  2026-11-02.

## Ownership and dependencies

`web/src/index.css` owns palette and effects. Shared UI controls consume those tokens;
features own their existing state and use semantic classes. No new HTTP rule, event
surface or composition root. The companion panel follows its own CSS owner and typed
platform/native boundaries in the accepted agents-and-panel design.

Existing Base UI/shadcn, Tailwind, Motion and Lucide cover the controls. The fonts ship
locally from `@fontsource/plus-jakarta-sans` and `@fontsource/jetbrains-mono` (approved by
Mohammed 2026-11-02); no font is loaded from a CDN. Reduced-motion and light-mode behavior
are preserved.

## Planned features

A feature the design shows and Core does not yet serve may show, inside the `Planned`
marker, with no invented value: a dash, an empty bar, a disabled control, never a guess or
a sample. No progress percentage, model state, memory metric or runtime capability is
computed in the client; each one shown is Core's answer or sits empty inside `Planned`.

## Delivery

Implement and inspect the web visual foundation (F-68), then complete the remaining
tasks of the accepted agents-and-panel plan: U-5, C-9, N-23, N-24, U-6, N-25, U-7, U-8.
Native and real-provider acceptance is recorded separately from browser fixtures.

The AI Studio migration (F-69 to F-77) follows its own
[plan](../plans/2026-11-02-ai-studio-ui-migration.md).
