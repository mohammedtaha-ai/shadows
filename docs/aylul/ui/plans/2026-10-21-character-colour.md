# Character colour: the companion state carries it

> **For agentic workers:** the task is U-1 in [`../stages.md`](../stages.md); a `frontend-builder`
> builds it. Steps use checkbox (`- [ ]`) syntax.

**Goal:** the companion window learns the companion agent's character colour from the native
companion state, so the character can be drawn in it.

**Spec:** "Character colour" under Agents in
[`docs/frontend/specs/2026-10-20-settings-dialog-design.md`](../../../frontend/specs/2026-10-20-settings-dialog-design.md).

**Where this sits:** Core stores `character_color` on the agent (web F-9). The native presence
reads it and publishes `appearance: { color }` in `companion-state` (native side, asked by an
AYLUL shared request). This task validates that field in `apps/desktop/src/platform/`. Drawing in
the colour (`DesktopCompanion` passing it to the renderer, the palettes) is the character side's.

## Task U-1: The companion state carries the character colour

**Files:**
- Modify: `apps/desktop/src/platform/companion-state.ts` (`CompanionState`, `acceptState`),
  `apps/desktop/src/platform/contract.yaml` (`acceptState` line and the new obligation)
- Test: `apps/desktop/tests/companion/native-state.test.ts`

**Interfaces:**
- Produces: `type CharacterColor = 'blue' | 'red' | 'green' | 'purple' | 'orange' | 'pink'`
  exported from `companion-state.ts`; `CompanionState` gains `color: CharacterColor`, always set.
- Consumes: the native state's optional `appearance: { color: string }` (its exact name follows
  the native side's answer; this section is amended if it differs).

- [ ] **Step 1: Write the failing tests** in `native-state.test.ts`:
  - `appearance colour is carried into the state`: a valid state with
    `appearance: { color: 'blue' }` gives `color === 'blue'`; each of the six values is kept.
  - `missing or unknown appearance draws purple`: no `appearance` (today's native and the legacy
    fixtures), `appearance: null`, `appearance: { color: 'teal' }` and
    `appearance: { color: 7 }` each give `color === 'purple'` and do not throw; the rest of the
    state is accepted as before.
  - `an older revision cannot change the colour`: current revision 5 blue, then revision 4 pink:
    the current state (blue) is returned.
- [ ] **Step 2: Run** from `apps/desktop/`: `node --test tests/companion/native-state.test.ts`;
  expected: FAIL.
- [ ] **Step 3: Implement.** An unknown colour is not an error: a newer native build may know a
  seventh colour, and the character must still draw. No other field's validation changes.
- [ ] **Step 4: Run** the test file and `npm run build`; PASS.
- [ ] **Step 5: Commit** `feat(desktop): the companion state carries the character colour`.
