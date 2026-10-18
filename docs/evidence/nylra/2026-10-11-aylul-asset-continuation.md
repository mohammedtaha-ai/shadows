# AYLUL third-master continuation

Probe sources and raw artifacts named here were removed from the tree on
2026-10-13; `git show 41b8e84:<path>` retrieves any of them.

Date: 2026-10-11, Asia/Aden. Baseline NYLRA `5801c9f`, clean before this work.
Mohammed liked idle-v3's drawing, requested continuation, then explicitly approved
Python cleanup of alpha=1 while preserving the art. Work stays in the NYLRA N0
trial/evidence area; no backend, account, product animation or original artwork changed.

## Established improvement

The previous v3 rejection recorded boundary alpha without its magnitude. Read-only
measurement now establishes all outermost nonzero edge pixels were alpha=1/255:
68 across all four edges (the historical 40 count considered only top/bottom).
No alpha>32 pixel occupied that border or the outer 8% margin. This is a weak
opacity residue, not a visibly cut-off hood. The earlier report remains historical.

One image_gen cleanup attempt (idle-v4) changed the hood appearance and retained
222 outer edge pixels at alpha=1. Do not substitute it for the user's preferred v3.
After explicit user authorization, clean-alpha.py removed exactly 38,726 alpha=1
pixels from v3 in a new file. Every RGB byte, every alpha>=2 byte and source-file
hash were verified unchanged. Canvas remains 1254x1254. Clean v3 has **zero**
nonzero alpha on all edges and throughout the outer 8%; the strict padding test passes.
Master source SHA-256: aedeed7c6f5b8e4f56450c6aeb20af5f9525a5428221a8b7b0a04edb89239915.
Cleaned SHA-256: 7aef454a43d3900381133014a0f866b305b80c58a78ba08652884c4351414d51.
Body opacity was intentionally not repainted: 301,248 pixels have alpha>=250, only
455 are exactly255. This pass closes the measured padding defect, not every earlier
solid-opacity condition or the full animated-character gate.

## Expression trials

Built-in image_gen generated listening-v4, blocked-v4 and listening-v5 from the
v3 design. The first listening expression remained too close to idle at small size;
v5 has wider round attentive eyes and brighter highlights. Blocked has concerned
eyes rather than an angry/malicious expression. Each has the same 1254x1254 canvas.
The same authorized alpha-only cleanup produced separate *-clean files; all have
zero alpha on edges and the outer8% margin. No RGB repaint/cropping/alignment was
performed by Python. Raw generated versions and all older trials remain intact.

Same canvas is **not** identical body/lighting. Excluding a normalized eye region
(x35–66%, y52–69%), read-only high-opacity RGB comparison found changes above8
levels in 20.07% of jointly solid body pixels for listening-v4, 17.99% for v5 and
15.65% for blocked-v4. Alpha/glow extent also differs. These are diagnostics, not
approved perceptual tolerances; some changes are subtle at64px, but an eyes-only
pixel-identity claim would be false. Expression trials remain candidates requiring
fixed-body/layer or reviewed visual-transition acceptance before product use.

## Preview and actual checks

Character preview (`docs/evidence/probes/2026-10-10-n0/character-preview.html` at `41b8e84`) compares original
v3, cleaned v3, expression trials and the unselected regenerated idle. It offers
dark/light/checkerboard backgrounds,64/96/128/256px display, manual three-state
switching on one fixed stage and optional4px CSS float. Reduced motion disables
float; page visibility pauses the hero. This is synthetic presentation, not a
Shadows/model state or a body-rig animation. It ships no product companion.

The local-only HTTP preview was started on127.0.0.1:5488. Browser inspection loaded
the actual PNGs; light background and small-size samples were viewed, and state
switch changed the hero to blocked. The motion button changed to its stop state.
Screenshots retained under artifacts/2026-10-11-release. This is browser preview
acceptance, **not** native-window/DPI, battery, CPU-budget or presentation-time proof.
Final browser checks confirmed all25 image elements loaded actual1254x1254 PNGs;
the selected listening-v5 hero measured64px when requested. A native indexed click
on an off-screen background control did not establish the style change; the
semantic button action then confirmed `surface light`. The optional motion has
exactly one image with the float animation style (the hero); comparison cards stay
static. Element.getAnimations was unavailable in the tool's read-only evaluation
surface, so verification used computed animation styles instead, without claiming
an animation-count/GPU performance measurement. Final rendered screenshot retained
at character-live-preview.png; the local preview tab is kept open for review.
Existing native performance evidence/budgets are not silently amended by this trial.

Commands (bundled Python, with no provider/API-key access):

```text
python clean-alpha.py native/ui/idle-v3.png native/ui/idle-v3-clean.png
python measure-master.py native/ui/idle-v3-clean.png native/ui/listening-v5-clean.png native/ui/blocked-v4-clean.png
python compare-states.py native/ui/idle-v3-clean.png native/ui/listening-v5-clean.png native/ui/blocked-v4-clean.png
python -m http.server 5488 --bind 127.0.0.1 --directory docs/evidence/probes/2026-10-10-n0
```

The first three are run from the probe directory; the server command from NYLRA
root. Exact generation prompts (`docs/evidence/probes/2026-10-10-n0/asset-prompts-v4-v5.md` at `41b8e84`),
source/output paths, SHA provenance, metrics and body-comparison JSON are retained.
No prior evidence/artwork overwritten, no product code, commit or push in this trial.

## Next closing trigger

Keep cleaned v3 as the idle master candidate preferred by the user. Before N1b,
review the expressions at64/96/128px, establish a fixed body/eye-layer workflow or
explicitly accept the measured state-lighting variation, then measure native motion
under the owner spec's reviewed budgets. Full N0, nine states, turning, mixed-monitor
and product lifecycle acceptance remain open. Reference/license questions retain
their existing owner; this experiment asserts provenance, not distribution rights.
