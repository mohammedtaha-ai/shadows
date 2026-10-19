# Approved AYLUL appearance and six additional expressions

Probe sources and raw artifacts named here were removed from the tree on
2026-10-13; `git show 41b8e84:<path>` retrieves any of them.

Publication amendment, 2026-10-12: Mohammed subsequently requested a commit and
push of the current NYLRA branch so another developer can continue. References
below to uncommitted/unpublished work describe the original preparation snapshot.

Preparation began 2026-10-11; final verification 2026-10-12, Asia/Aden.
Baseline `5801c9f`, branch `codex/windows-shell-n0`. The user explicitly approved
the displayed idle/listening/blocked appearances, then selected preparation of the
remaining six expressions instead of immediate Windows product integration.

## Three approved sources

Copied without pixel changes into canonical pack `assets/aylul/states/v1/manifest.json` (now in the local, untracked [NYLRA archive](../../../assets/aylul/README.md)):
idle-v3-clean -> idle.png, listening-v5-clean -> listening.png, blocked-v4-clean ->
blocked.png. Their approved hashes remain identical to the previously inspected
source images. Appearance approval accepts the displayed lighting/glow variation;
it does not establish native animation, event bindings or distribution acceptance.
The delivery spec was amended in place to preserve this choice as the appearance
reference. No approved drawing was regenerated or overwritten.

## Six prepared candidates

Candidate pack `assets/aylul/candidates/2026-10-11/manifest.json` (local [NYLRA archive](../../../assets/aylul/README.md)) contains:

| Expression | Selected source | Visual distinction |
| --- | --- | --- |
| Happy | happy-v1-clean | Bright upward smiling crescent eyes |
| Speaking | speaking-v1-clean | Conversational oval eyes with three left-side voice accents |
| Thinking | thinking-v1-clean | Asymmetric thoughtful eyes |
| Working | working-v2-clean | Flat focused violet eyes, without the first trial's face arc |
| Success | success-v2-clean | Smiling crescents with two restrained lavender sparkles |
| Sleeping | sleeping-v1-clean | Dimmer closed relaxed eyelid arcs |

Eight built-in image_gen calls: six initial expressions, then focused corrections
for working/success. Input was the approved idle appearance; the first happy/sleeping
calls used its byte-identical cleaned evidence path, the rest its canonical copy.
Full prompts and output filenames (`docs/evidence/probes/2026-10-10-n0/six-expression-prompts.json` at `41b8e84`)
are retained; internal generator model version was not exposed. No provider API
key or MiniMax calls were used. No Coucou artwork was imported.

The same explicitly authorized Python cleanup removes only alpha=1 in new files,
preserving all RGB bytes, alpha>=2, canvas and original source hashes. Each operation
checks those invariants after reopening the saved PNG and writes provenance JSON.
Raw generated versions and failed first working/success trials remain retained.

## Observations and corrections

Working-v1 zoomed the body and had 2,924 nonzero-alpha pixels in the outer8% margin,
including 556 above alpha32. Its face arc was also more prominent than intended.
Success-v1 had 84 low-alpha pixels in that margin, none above32. Those were not
silently removed by expanding the authorized cleanup threshold. Corrective edits
returned working to a source-sized focused expression and bounded success glows.
Their v2 derivatives replace only those candidate selections, not the approved pack.

All six selected final PNGs are 1254x1254 and have zero alpha on every edge and
the entire outer8% margin. Final metrics (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/six-final-state-metrics.json` at `41b8e84`)
and both failed first-trial measurements are retained. Candidate copy hashes match
the selected evidence PNGs. These checks prove file/padding integrity, not identical
body pixels or human approval. New expressions retain generated lighting variation.

## Preview and actual validation

Updated preview (`docs/evidence/probes/2026-10-10-n0/character-preview.html` at `41b8e84`) shows nine controls,
three approved labels and six review labels. Added compact nine-card comparison;
detail mode retains64/96/128 samples, background choices and synthetic float.
Actual browser checks: nine cards, nine enabled state controls, all PNGs loaded;
each of the six controls selected its correct final1254px image. Compact mode and
light background worked. Rendered review sheet (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/nine-state-review.png` at `41b8e84`)
was saved and the fresh preview tab kept open. This is browser/static-asset evidence,
not a Windows companion renderer/performance or real product-state assertion.

The prior local preview server had stopped. Reconnecting exposed the browser's
connection-refused error page; navigation to that generated data URL was policy
blocked. A combined hidden background-start shell command was rejected before
execution (`blocked by policy`, no more specific reason supplied). A narrower
foreground Python server bound only127.0.0.1:5488 succeeded; host socket and HTTP200
were verified. The stale tab remained on its error page, so a fresh ordinary HTTP
tab on the same in-app browser loaded the restored server successfully. No firewall,
certificate verification, browser URL policy or global setting was weakened.

Final file checks verify the three approved hashes, six candidate hashes/copies,
their equal canvas and padding. Actual final validation passed35 local/sibling
documentation targets, backend fixture equality, diff whitespace and exact RGB/
alpha-cleanup assertions for all six selected sources. No product source, backend files, commit, push or
deployment changed in this appearance task; prior uncommitted trial work is preserved.

## Next trigger

Developer handoff: start at assets/aylul/README.md and the current delivery spec/
initial plan. Preserve the three approved PNG hashes. The six candidate files are
prepared and visibly labelled for Mohammed's review; do not silently approve them
or redraw the approved master. The preview is a local-only HTML file; run its
documented Python HTTP server from the repo root to reproduce it. The next product
slice is the actual Windows renderer/event integration after its remaining gates;
do not infer implemented voice, successful work outcomes or project authority from
the static expression names.

Mohammed reviews the six candidate expressions at the provided sizes/backgrounds.
Accepted candidates may then join the canonical pack. Native motion/performance,
actual capture/playback/outcome bindings, directions and release/distribution
remain their existing independent gates. No preview click constitutes a successful
task, capture state or human approval of a Shadows action.
