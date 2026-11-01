# Neon clarity implementation plan

> Execute inline with superpowers:executing-plans. Current checkout only; serialized builds.

**Goal:** Apply Mohammed's chosen neon visual hierarchy without changing page arrangement.

**Architecture:** Palette/effects remain in index.css. Shared controls consume semantic
tokens. Agent form owns its unsaved indicator; the permission card owns waiting state.
The existing desktop plan owns the companion panel and all native behavior.

**Tech stack:** Existing React, Base UI/shadcn, Tailwind, Motion and Lucide; no new dependency.

### F-68

- [x] Update dark/light palettes, luminous primary/selected controls and surface hierarchy.
- [x] Apply the shared visual treatments to the agent and permission surfaces; show unsaved
  agent edits from existing form state, with meaningful focused tests.
- [x] Run agent-form, outfit-picker and permission-card tests; typecheck, lint, build,
  codemap and architecture. Inspect browser screenshots in both themes and reduced motion.
- [x] Update changed contracts/stages and commit explicit files.

Touch: web/src/index.css, web/src/components/ui/button.tsx,
web/src/app/agents/agent-settings.tsx, web/src/app/agents/agent-form.test.tsx,
web/src/app/agents/outfit-picker.tsx, web/src/app/agents/agent-extensions.tsx,
web/src/app/agents/contract.yaml, web/src/app/conversation/permission-card.tsx,
web/src/app/conversation/contract.yaml, docs/frontend/stages.md, this plan and its owner spec.

### Remaining desktop work

Use the exact Touch and Done when of docs/aylul/{ui,native,character}/stages.md, in the
dependency order in the owner spec. Each task has its own RED/GREEN evidence and commit.
The companion panel inherits the approved neon direction in its own panel.css.
