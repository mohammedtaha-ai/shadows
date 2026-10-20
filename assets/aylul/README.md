# AYLUL appearance sources

`character-colors.json` is the shared character colour table, owned by the desktop
UI side (Masada). The web swatches and renderer palette read this one file. To add
a colour, append its id, English label and hex swatch, then add its id to the web
order test; optional tone overrides replace derived tones. The base entry keeps
the renderer's original complete palette.

Relocation note (2026-10-18): the renderer and this provenance moved into Shadows.
The ignored historical archives described below remain under
`E:/Globalprojects/nylra/archive/`; they were not copied or deleted. They are not
build dependencies. A future artwork/archive transfer must inventory the selected
files explicitly instead of importing all ignored data.

The current Canvas companion was extracted from the supplied `ayl.html` into
`apps/desktop/src/features/companion/`. Its design and implementation plan live in
`docs/aylul/character/`. The renderer does not load the historical PNGs.

After the renderer was accepted, the original standalone HTML was moved on
2026-10-17 to ignored local `archive/aylul-source-2026-10-17/ayl.html` so it no
longer remains loose at the repository root. Its SHA-256 remains
`103a5ba2765c4a0f81f9b93051d2883fef1dc79a7e49dc8cb4ee0d0ff76a8bae`.
This archive is a local reference, not an application build dependency or a file
distributed by Git. The extracted TypeScript renderer is the current source.

At the user's request on 2026-10-16, the legacy PNG expressions, animation packs,
candidate images and reference board were moved to the local directory
`archive/aylul-legacy-2026-10-16/`. Complete packs retain their metadata, previews
and generation tools. All moved files were verified by SHA-256. `/archive/` is
ignored by Git and is not distributed with a fresh checkout. Historical versions
remain recoverable from Git history; no history was rewritten.

Earlier evidence documents describe the assets at their original paths and dates.
Their PNG approval does not establish acceptance of the new Canvas renderer.
