# Neon clarity

Approved direction: Mohammed chose neon on 2026-11-01, asked for implementation while
away, and requested the remaining agent-panel tasks in the same run.

The existing navigation, page layout and Core behavior stay authoritative. The visual
system makes controls, selected items, section headings and real operation states easier
to distinguish: dark violet surfaces, bright purple primary actions, restrained luminous
edges and readable secondary text. Light mode retains equivalent hierarchy. Color never
replaces the text or icon of an operation state.

## Approaches

- Quiet premium: subtle purple, little glow. Considered; Mohammed chose the stronger neon.
- Neon clarity: luminous primary/selected controls and layered surfaces, clear text and
  bounded motion. Selected.
- Full-screen effects: continuous glow animations and decoration. Not needed for the
  requested clarity; it would obscure content and increase rendering work.

## Ownership and dependencies

`web/src/index.css` owns palette and effects. Shared UI controls consume those tokens;
features own their existing state and use semantic classes. No new HTTP rule, event
surface or composition root. The companion panel follows its own CSS owner and typed
platform/native boundaries in the accepted agents-and-panel design.

Existing Base UI/shadcn, Tailwind, Motion and Lucide cover the work. No additional library
is required. Reduced-motion and light-mode behavior are preserved. No artificial progress
percentages, model state, memory metrics or runtime capabilities are introduced.

## Delivery

Implement and inspect the web visual foundation (F-68), then complete the remaining
tasks of the accepted agents-and-panel plan: U-5, C-9, N-23, N-24, U-6, N-25, U-7, U-8.
Native and real-provider acceptance is recorded separately from browser fixtures.
