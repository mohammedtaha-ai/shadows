# The design stage says how many files a structure writes

- Status: asked
- Asked by: Masada, 2026-10-29
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 4 `structure-count`)

## Need

Before the person confirms "Create structure", the screen says how many files it
will write. Today `web/src/app/design/create-structure.tsx::fileCount` reads every
page of every part's children and counts the root `SHADOWS.md`, one file per part
and each part's listed files, restarting when the map's revision changes. That
count is Core's rule about what `create_structure` writes. The stage read must
carry it, so the walk can go.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/projects/{id}/stage` | nothing | adds `structure_file_count: integer \| null`: the files `POST /api/projects/{id}/structure` would write for the map at this stage's revision; `null` when the stage cannot create a structure |

## Errors

| When | The screen shows |
|---|---|
| The count is `null` | no count, and the confirm stays as the stage allows |
| The map changes between read and confirm | the existing 409 handling: re-read the stage |

## Backend answer

## Frontend reply

## Ruling
