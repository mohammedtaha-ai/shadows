# Images in a message, and the `+` menu

- Status: accepted
- Asked by: Mohammed, 2026-10-22 (the controller wrote it from his ruling: `Threads` owns
  attachments; images first; the `+` menu with Slash commands and Plugins; folders and
  connectors later)
- Screen: the composer in `web/src/app/conversation/` (also the AYLUL desktop, which packages
  the same client). Owner specs: [domain model §4.2 Attachment](../../superpowers/specs/2026-09-21-domain-model-design.md#attachment),
  [harness controls §12.11](../../superpowers/specs/2026-09-24-harness-controls-design.md#1211-web-client),
  [slash menu §21](../../superpowers/specs/2026-10-05-slash-menu-design.md)

## Need

Mohammed pastes a screenshot into the message box, drops an image on it, or picks one from
`+` → Add photos (Ctrl+U). Each image uploads at once and shows as a thumbnail he can remove.
He sends text with images, or images alone, and the model sees them. The `+` menu also opens
the `/` menu and, in an AYLUL conversation, the agent's Plugins & Skills settings.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `POST /api/threads/{id}/attachments` | multipart: `command_id`, one `file` | `{ id, media_type, byte_len, name }` |
| `GET /api/threads/{id}/attachments/{attachment_id}` | — | the bytes, with their media type (thumbnails and the transcript) |
| `POST /api/threads/{id}/turns` | today's body plus `attachments: [string]` (ordered ids; `prompt` may be empty when it is not empty) | unchanged |
| the queued-message and steer calls | their body plus `attachments: [string]` | unchanged |
| a thread's entries (read and stream) | — | a user entry carries `attachments: [{ id, media_type, name }]` |
| `POST /api/threads/{id}/session` | unchanged | `capabilities.image: bool`, from the harness |

## Errors

| When | The screen shows |
|---|---|
| Not an image Core accepts, by content | the thumbnail turns into an error with "This file type is not supported" |
| Over the size limit | the thumbnail error names the limit |
| The harness does not take images (`capabilities.image` false, or the turn refused) | Add photos and paste are disabled with that reason; a refused send keeps the text and images |
| An id from another thread, or unknown | the send is refused; the text and images stay |
| A replay with a different body (`CommandConflict`) | the message stays unsent; no retry with the same id |

## Backend answer

As asked. Owner: `Threads` (records and bytes, the upload and read routes, removal and fork),
`Turns` (the ids in the turn, queue and steer commands and their fingerprints; neutral image
blocks in the one prompt composition), the ACP adapter in `shadows-agent` (neutral blocks to
ACP `image` blocks; the prompt capabilities kept from `initialize`), `Harness` (capabilities in
the session's answer). Built in B-11 after A-18 reshapes turns.

## Frontend reply

Accepted: F-24 (the `+` menu with Slash commands and Plugins, which needs no backend) and F-25
(paste, drop, picker, thumbnails and sending, on B-11).
