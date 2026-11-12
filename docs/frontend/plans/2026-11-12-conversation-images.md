# Conversation images look like Claude's composer

**Ruling (Mohammed, 2026-11-12):** an image in the conversation looks and behaves like
Claude's own composer. One task, F-114 in [`stages.md`](../stages.md); this whole plan is its brief.

**Starts after** the frontend-fixer's `crossOrigin="anonymous"` fix is committed (it edits
`pending-images.tsx`, `entry.tsx`, `waiting-messages.tsx` and their tests). Every `<img>` this
task draws, thumbnail or large, keeps `crossOrigin="anonymous"`.

No Core change: the images, their upload state and their URLs already come from
`web/src/api/` (`attachmentUrl`) and `upload-images.ts`. This task changes how they are drawn.

## What the person sees

**Composer (`PendingImages`).** Each chosen or pasted image is a small square tile
(`size-16`, `rounded-lg`, `overflow-hidden`, image `object-cover`), in a wrapping row.

- A round ✕ button sits on the tile's top-right corner (`absolute -top-1.5 -end-1.5`,
  `size-5 rounded-full`, lucide `X` icon `size-3`), always visible. Accessible name
  `Remove <file name>`. Disabled while `inFlight`, as today.
- No status text under the tile: "Uploaded", "Uploading..." and "Ready to upload on send" are gone.
- Uploading: the tile dims (`opacity-60`) and shows a small centred spinner (lucide
  `Loader2`, `animate-spin`, `motion-reduce:animate-none`, `aria-hidden`); the tile `li` carries
  `aria-busy="true"`.
- Failed: the tile gets a destructive ring and a round retry button centred on it (lucide
  `RotateCw`), accessible name `Retry <file name>`, `title` = Core's message, shown only when
  `threadId !== null` and disabled while `inFlight` (today's rule). Core's message is also shown
  as one line under the tile row (`<file name>: <message>`, `text-xs text-destructive-foreground`),
  so the reason stays readable at this size.
- Uploaded, or local in a draft (no thread yet): the plain tile.
- Each tile `li` carries `data-upload="local" | "uploading" | "uploaded" | "failed"`, derived
  exactly as today's text was: `error` → failed, `attachment` → uploaded, `threadId === null` →
  local, else uploading. Tests read this attribute instead of the old text.

**Sent message (`Entry`, `UserMessage`).** The images keep their size (`max-h-48 max-w-64`,
`object-contain`) and lose the `<a href target=_blank>` link; clicking one opens the viewer.

**Waiting message (`WaitingMessages`).** The images keep `size-24 object-cover`; clicking one
opens the viewer. No ✕ on them: they go with the message's own Remove button, as today.

**The viewer.** A click (or Enter/Space) on any thumbnail opens a dialog over the page holding
the whole image at its natural aspect, fitted to the screen (`max-h-[calc(100dvh-4rem)]
max-w-[calc(100vw-4rem)] object-contain`), on a transparent, borderless dialog body. It closes
on Esc, a click outside and its close button, and focus returns to the thumbnail that opened it.

## The parts

### `ImageZoom` (new, `web/src/app/conversation/image-zoom.tsx`)

One exported component; it owns the thumbnail button and its viewer dialog.

```ts
export function ImageZoom(props: {
  alt: string
  /** A URL from attachmentUrl; or omit and pass `image`. */
  src?: string
  /** Sets and cleans up a node's src (the composer's local object URL). Used for both images. */
  image?: (node: HTMLImageElement | null) => void | (() => void)
  /** Classes for the thumbnail <img> (its size and fit). */
  className?: string
}): JSX.Element
```

- The thumbnail is a `<button type="button">` with accessible name `View <alt>`, holding an
  `<img alt={alt} crossOrigin="anonymous">` with `src` or `ref={image}`. Focus ring:
  `focus-visible:ring-3 focus-visible:ring-ring/50`.
- The viewer uses the existing `Dialog`, `DialogContent` and `DialogTitle` from
  `web/src/components/ui/dialog.tsx` (no new dialog, no edit to that file), controlled by local
  `open` state. `DialogTitle` is `sr-only` with the alt text, so the dialog is named. The default
  close button stays (`showCloseButton`), name `Close`. Focus returns to the thumbnail button:
  pass `finalFocus` with a ref to it if Base UI does not already.
- The large `<img>` uses the same `src` or `image` callback (a second object URL for a local file
  is made and revoked by the same callback), `alt={alt}`, `crossOrigin="anonymous"`.
- It holds no upload state and no rule.

### `PendingImages` (`pending-images.tsx`)

Props unchanged: `{ images, threadId, inFlight, onRetry, onRemove }`. `ImagePreview` becomes the
`image` callback handed to `ImageZoom` (its object URL and `attachmentUrl` logic and revoke stay as
they are). The tile layout above replaces the text and text buttons.

### `Entry` and `WaitingMessages`

Each attachment `<img>` becomes `<ImageZoom src={attachmentUrl(...)} alt=... className=.../>`
with today's alt texts (`image.name ?? 'Attached image'`, `Queued image`) and sizes. `Entry`
keeps `loading="lazy"`: add an optional `loading?: 'lazy'` prop to `ImageZoom` for the thumbnail
only, and list it in the contract.

## Tests

The viewer renders in a portal: query `document.body`, not `app.container`, for the dialog.

`web/src/app/conversation/image-zoom.test.tsx` (new):
- `the thumbnail opens the whole image in a named dialog` — button `View a.png`; after the click,
  a `role="dialog"` named `a.png` holds an `img[alt="a.png"][crossorigin="anonymous"]` with the same src.
- `Esc and the close button close the viewer and focus returns to the thumbnail` (`it.each`).

`composer-images.test.tsx` (modify):
- Every `app.text().includes('Uploaded')` wait becomes a wait on every tile in
  `[aria-label="Images"]` having `data-upload="uploaded"` (one local helper in the file).
- `%s uploads immediately with a removable thumbnail` also checks: no text `Uploaded`, the
  `Remove screen.png` button sits inside the tile `li`, the image is inside a `View screen.png` button.
- New: `an uploading image is a quiet busy tile with no status text` — the tile has
  `data-upload="uploading"` and `aria-busy="true"`, and the page has no `Uploading` text.
- `a failed upload keeps text and images with Core reason and retries the same upload command`
  also checks the tile has `data-upload="failed"` and the `Retry screen.png` button is inside it.
- New: `clicking a composer tile opens the image large`.
- `the transcript renders ordered message images from the thread byte route` also checks no
  `a[href*="/attachments/"]` remains; new: `clicking a sent image opens it large`.

`waiting-messages.test.tsx` (modify):
- New: `a waiting message image opens large and has no remove button on it` — a `View Queued image`
  button opens the dialog; the row's buttons are the image buttons, then `Send now`, `Remove`.

## Contract and code map

- `web/src/app/conversation/contract.yaml`, `image_attachments`: the `owner` names `ImageZoom`
  (thumbnail button and viewer); the `PendingImages` line describes the tile, the corner ✕, the
  quiet upload state and `data-upload`; add an `ImageZoom` line with its props; `previews` says
  sent and queued images open the viewer and are no longer links; the `tests` list adds the new names.
- `docs/frontend/codebase/README.md`: a row for `web/src/app/conversation/image-zoom.tsx`
  ("Opens one conversation image large in a dialog", contract `web/src/app/conversation/contract.yaml`).

## Done

From `web/`: `npx vitest run src/app/conversation/image-zoom.test.tsx
src/app/conversation/composer-images.test.tsx src/app/conversation/waiting-messages.test.tsx`,
`npm run typecheck`, `npm run lint`, `npm run build`,
`npx vitest run src/codemap.test.ts src/architecture`. Mohammed's look in the browser is the
visual acceptance; there is no mockup file.
