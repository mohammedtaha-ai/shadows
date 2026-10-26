# Character colour: the colour table, the renderer's palette, the companion state

> **For agentic workers:** the tasks are U-1, U-2 and U-3 (U-2 in [`../stages.md`](../stages.md);
> U-1 and U-3 at `1a2b601`, delivered by PR #40); a
> `frontend-builder` builds one task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** AYLUL's character is drawn in the companion agent's colour, from one colour table that
the web swatches and the renderer both read; adding a colour is one table entry plus its id in the
order test.

**Spec:** "Character colour" under Agents in
[`docs/frontend/specs/2026-10-20-settings-dialog-design.md`](../../../frontend/specs/2026-10-20-settings-dialog-design.md).

**Ownership:** Mohammed's ruling (2026-10-21): the colour table (`assets/aylul/character-colors.json`)
and the renderer's palette are Masada's, built by `frontend-builder`, although `assets/aylul/` and
`features/companion/` are otherwise the character side's. These tasks touch only what the palette
needs: no shape, motion, expression or outfit changes.

**Order:** U-2 (the table and the palette function) first: web F-9 needs the table. U-3 (the
renderer reads the palette) needs U-2. U-1 (the companion state carries the id) can run any time;
U-3 then wires it into `DesktopCompanion`.

**Where the id comes from:** Core stores `character_color` on the agent as an opaque string,
default `"purple"` (web F-9). The native presence publishes it unchanged in `companion-state` as
`appearance.color` (native side, asked by an AYLUL shared request). Only the table gives an id
meaning; any id the table does not hold draws purple.

## Task U-2: The colour table and the palette function

**Files:**
- Create: `assets/aylul/character-colors.json`, `apps/desktop/src/features/companion/renderer/palette.ts`
- Modify: `apps/desktop/tsconfig.json` (`resolveJsonModule: true`), `apps/desktop/vite.config.ts`
  (only if the dev server refuses the file outside `apps/desktop/`: `server.fs.allow` gains that
  one file), `apps/desktop/src/features/companion/contract.yaml` (export `paletteFor`, the two
  obligations below), `assets/aylul/README.md` (one paragraph: the table, its owner, how to add a
  colour)
- Test: `apps/desktop/tests/companion/palette.test.ts` (new; add it to `test:companion:unit` in
  `apps/desktop/package.json`, which is then also touched)

**The table** (`assets/aylul/character-colors.json`), the only place a colour is named:

```json
{
  "base": "purple",
  "colors": [
    { "id": "blue",   "label": "Blue",   "swatch": "#3b82f6" },
    { "id": "green",  "label": "Green",  "swatch": "#22c55e" },
    { "id": "purple", "label": "Purple", "swatch": "#a855f7", "tones": { "…": "every tone below" } },
    { "id": "orange", "label": "Orange", "swatch": "#f97316" },
    { "id": "pink",   "label": "Pink",   "swatch": "#ec4899" }
  ]
}
```

- `base` names the entry whose `tones` is complete; it holds today's literals as `#rrggbb` (the
  alpha of each use stays in the renderer's code).
- Any other entry's tones are **derived**: each base tone keeps its HSL saturation and lightness
  and its hue turns by `hue(entry.swatch) - hue(base.swatch)`. An entry may carry its own `tones`
  with some keys; those keys override the derived ones (for a tone that looks wrong after review).
- The base tones, with today's values (the key is the name `paletteFor` returns):

| Key | Value | Where it is used today |
|---|---|---|
| `glow` | `#a855f7` | back glow, working back ring, body rim, dome highlight, paw shadow, thinking `?` shadow, notification fill, crystal lower facet 0 |
| `glowSoft` | `#c084fc` | inner cavity bounce, back orbital ring, paw stroke, pedestal ring, mask stroke, working ring, crystal upper facet 3 |
| `glowPale` | `#d8b4fe` | dome sheen, hood rim highlight, scene particles |
| `glowMist` | `#e9d5ff` | thinking `?` fill |
| `rimLeft` | `#a78bfa` | left hood rim |
| `spark` | `#e879f9` | eye mid glow, crystal bounce, working ring, success back glow, notification ring, pedestal inner ring, speaking shadow, crystal upper facet 2 |
| `sparkDeep` | `#d946ef` | crystal glow, mouth shadow, front ring shadow, listening and working shadows |
| `sparkSoft` | `#f0abfc` | speaking strokes, working ring, crystal lower facet edge |
| `sparkPale` | `#f5d0fe` | mouth fill, front orbital ring, listening strokes, notification stroke, crystal upper facet 1 |
| `core` | `#fdf4ff` | eye core, crystal upper facet 0 |
| `bloom` | `#9333ea` | eye outer bloom, crystal lower facet 1 |
| `crystalDeep` | `#7e22ce` | crystal lower facet 2 |
| `crystalDark` | `#581c87` | crystal lower facet 3 |
| `innerRim` | `#c891dc` | inner hood rim line (`rgba(200, 145, 220, …)`) |
| `sleep` | `#c4b5fd` | sleeping `z` fill |
| `sleepGlow` | `#818cf8` | sleeping `z` shadow |
| `bodyLit`, `bodyMid`, `bodyCore`, `bodyRim` | `#2a1f3d`, `#150f21`, `#08050d`, `#1e1136` | lower body gradient |
| `occlusion` | `#030107` | hood shadow on the body |
| `extrusion` | `#120c1c` | hood back wall |
| `hoodLit`, `hoodMid`, `hoodFold`, `hoodBase`, `hoodBounce` | `#43325c`, `#2b1f3d`, `#171122`, `#100b18`, `#2d1d42` | hood gradient |
| `seamLine` | `#0c0814` | cloak seam lines |
| `wing` | `#1b1426` | hood side folds |
| `bevelTop`, `bevelMid`, `bevelBottom` | `#2e1d3a`, `#1a1024`, `#3b244b` | inner hood bevel |
| `voidCore`, `voidDeep`, `voidRim` | `#0a0513`, `#040208`, `#150c24` | face cavity |
| `pawLit`, `pawMid`, `pawDark` | `#4c356e`, `#1d142b`, `#09060e` | paws |
| `maskLit`, `maskMid`, `maskDark` | `#1e162e`, `#2d2144`, `#120d1c` | mask outfit veil |
| `brow` | `#06030b` | angled brow mask |
| `floor` | `#05030a` | pedestal floor shadow |

- Not tones, never changed by a colour: gold metal (`#fde047`, `#d97706`, `#78350f`, `#fef08a`,
  `#451a03`, `#fbbf24`), the rose-gold brow seam (`rgba(186, 132, 174, …)`), white, every
  `blocked` colour (rose `#f43f5e` family, the red crystal shades), every `success` gold, and
  every colour in `outfits.ts` and the `starry` sparkles.

**Interfaces** (`palette.ts`):
- `type Tone = keyof` the base tones (the keys above); `type Palette = Readonly<Record<Tone, string>>`
  (`#rrggbb`).
- `paletteFor(id: string | null | undefined): Palette`: the table entry with that id, its tones
  derived and overridden as above; an id not in the table, empty or missing gives the base
  palette. Results are cached per id (one object per colour, no work per frame).
- `rgba(tone: string, alpha: number): string`: `"rgba(r, g, b, a)"` with today's spacing, so a
  base-palette string equals today's literal character for character.

- [ ] **Step 1: Write the failing tests** in `palette.test.ts`:
  - `the base palette is today's colours`: `paletteFor('purple')` equals the table's purple
    `tones` exactly, and holds every key of the table above (the test lists them).
  - `an unknown or missing id draws purple`: `paletteFor('red')`, `paletteFor('teal')`,
    `paletteFor('')`, `paletteFor(null)` and `paletteFor(undefined)` are the same object as
    `paletteFor('purple')`.
  - `every colour in the table has a full palette`: for each entry of `colors`, every tone is
    `#rrggbb`; a non-base entry's `glow` differs from purple's.
  - `a derived tone keeps lightness and saturation`: for each non-base entry and each tone, the
    HSL lightness and saturation equal the base tone's within 1 percentage point, unless the
    entry overrides it. The hue is checked only where 8-bit RGB can carry it: when the tone's
    chroma (max - min of its R, G, B, 0-255) is at least 32, the hue differs by the swatch hue
    difference (mod 360) within 2 degrees; darker or greyer tones (such as `occlusion`) are
    checked on lightness and saturation only, since rounding to `#rrggbb` moves their hue by
    several degrees.
  - `an override wins`: a test table with `{ id: 'x', swatch: '#3b82f6', tones: { glow: '#123456' } }`
    gives `glow === '#123456'` and the other tones derived (`paletteFor` takes an optional table
    argument for this test; the product passes none).
  - `rgba keeps today's text`: `rgba('#a855f7', 0.42)` is `"rgba(168, 85, 247, 0.42)"`.
  - `the table's entries are complete`: ids match `/^[a-z][a-z0-9-]{0,31}$/` and are unique, each
    `label` is non-empty, each `swatch` is `#rrggbb`, and `base` names an entry with `tones`.
- [ ] **Step 2: Run** from `apps/desktop/`: `node --test tests/companion/palette.test.ts`; FAIL.
- [ ] **Step 3: Implement** the table and `palette.ts` (the HSL conversion is ours, a few lines;
  no dependency). Nothing in the renderer uses it yet.
- [ ] **Step 4: Run** the test file and `npm run build`; PASS.
- [ ] **Step 5: Commit** `feat(desktop): one colour table for AYLUL's character`.

## Task U-1: The companion state carries the character colour

**Files:**
- Modify: `apps/desktop/src/platform/companion-state.ts` (`CompanionState`, `acceptState`),
  `apps/desktop/src/platform/contract.yaml` (`acceptState` line and the new obligation)
- Test: `apps/desktop/tests/companion/native-state.test.ts`

**Interfaces:**
- Produces: `CompanionState` gains `color: string`, always set: the native `appearance.color`
  when it is a string of 1 to 32 characters, else `'purple'`. The id is opaque here: this module
  holds no list of colours. The renderer resolves it against the colour table and draws purple
  for an id it does not know, so adding a colour changes nothing in this module.
- Consumes: the native state's optional `appearance: { color: string }` (its exact name follows
  the native side's answer; this section is amended if it differs).

- [ ] **Step 1: Write the failing tests** in `native-state.test.ts`:
  - `appearance colour is carried into the state`: a valid state with
    `appearance: { color: 'blue' }` gives `color === 'blue'`; an id no table holds today
    (`appearance: { color: 'teal' }`) is kept as `'teal'` too (the renderer, not this module,
    falls back).
  - `missing or malformed appearance reads as purple`: no `appearance` (today's native and the
    legacy fixtures), `appearance: null`, `appearance: { color: 7 }`, `appearance: { color: '' }`
    and a 33-character id each give `color === 'purple'` and do not throw; the rest of the state
    is accepted as before.
  - `an older revision cannot change the colour`: current revision 5 blue, then revision 4 pink:
    the current state (blue) is returned.
- [ ] **Step 2: Run** from `apps/desktop/`: `node --test tests/companion/native-state.test.ts`;
  expected: FAIL.
- [ ] **Step 3: Implement.** A malformed colour is not an error: the character must still draw. No other field's validation changes.
- [ ] **Step 4: Run** the test file and `npm run build`; PASS.
- [ ] **Step 5: Commit** `feat(desktop): the companion state carries the character colour`.

## Task U-3: The renderer draws in the palette; purple is unchanged

**Files:**
- Create: `apps/desktop/tests/companion/fixtures/purple-frames.json` (step 1, from today's code)
- Modify: `apps/desktop/src/features/companion/types.ts` (`CharacterFrame` and `MotionInput`
  gain `color?: string`), `apps/desktop/src/features/companion/animation/model.ts` (copy
  `input.color` into the frame), `apps/desktop/src/features/companion/renderer/character.ts`,
  `renderer/eyes.ts`, `renderer/crystals.ts`, `renderer/hands.ts`, `renderer/effects.ts`,
  `renderer/scene.ts` (each tone literal of the U-2 table becomes `palette.<tone>` or
  `rgba(palette.<tone>, a)`; the palette is resolved once per frame in `renderCharacter` and
  `renderScene` with `paletteFor(frame.color)` and passed down as one argument),
  `apps/desktop/src/features/companion/DesktopCompanion.tsx` (`input.color = state?.color`),
  `apps/desktop/src/features/companion/contract.yaml`, `apps/desktop/tests/companion/fixture.tsx`
  (`render(expression, outfit, color?)` and `scene(expression, color?)`)
- Test: `apps/desktop/tests/companion/renderer.spec.ts`, `apps/desktop/tests/companion/desktop-window.spec.ts`
- Not touched: `renderer/outfits.ts`, `studio/` (an absent `color` is purple, so the studio and its
  PNG export keep drawing today's look)
- Small follow-up: `drawTrue3DCrystal` also serves the unchanged outfit renderer; its new
  palette argument is optional and defaults to the base, while character callers pass their
  resolved palette. Review exports are generated into ignored `output/playwright/character-colours/`
  by the opt-in `export colour review stills` test in the listed renderer spec.

- [ ] **Step 1: Record today's pixels, before any renderer change.** Add `scene()` to the
  fixture and a spec `record purple frames` (skipped unless `RECORD_PURPLE=1`) that writes, for
  each of the nine expressions × ten outfits, plus each expression through `scene()`, the mean
  premultiplied RGBA of each 4x4 pixel block (brotli, base64) into `fixtures/purple-frames.json`.
  A hash would tie the reference to one machine's rasterizer. Run it once on the unchanged
  renderer and commit the file alone: `test(desktop): record today's character pixels`.
- [ ] **Step 2: Write the failing tests** in `renderer.spec.ts`:
  - `purple draws today's pixels`: every recorded frame, drawn with `color` absent, `'purple'`
    and `'red'` (not in the table), matches its recorded blocks with no channel more than 24 off.
  - `each colour in the table draws its own look`: for every table entry other than the base,
    `render('idle', 'default', id)` differs from purple's hash, keeps transparent corners and
    draws as many or more visible pixels than purple minus 1%.
  - `signals keep their colours in every colour`: for every entry, a pixel sampled on the
    `blocked` warning triangle is the rose `#f43f5e` (±2 per channel) and one on the `success`
    star is gold, as in purple.
  - In `desktop-window.spec.ts`, `the companion window draws the state's colour`: the desktop
    entry given a native state with `appearance: { color: 'blue' }` draws a canvas whose sampled
    glow pixel is blue's `glow` hue (±4 degrees), and with no `appearance` purple's.
  - `same inputs produce same pixels` and `all expressions and outfits draw without global engine`
    still pass.
- [ ] **Step 3: Run** from `apps/desktop/`: `npx playwright test tests/companion/renderer.spec.ts
  tests/companion/desktop-window.spec.ts`; FAIL (no colour yet).
- [ ] **Step 4: Implement** the literal-to-tone pass exactly as the U-2 table maps it; no
  literal outside the table changes; no draw call is added or removed.
- [ ] **Step 5: Run** the spec, `npm run test:companion:unit`, `npm run build`; PASS. Export one
  still per colour from the studio fixture (`render('idle','default',id)` for each table entry,
  plus `working` and `blocked` in blue) and put the screenshots in the report: they are what
  Mohammed reviews.
- [ ] **Step 6: Commit** `feat(desktop): AYLUL is drawn in the companion agent's colour`.
