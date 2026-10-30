# The design stage says how many files a structure writes

- Status: accepted
- Asked by: Masada, 2026-10-29
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 4 `structure-count`)

## Need

Before the person confirms "Create structure", the screen says how many files it
will write. Today `web/src/app/design/create-structure.tsx::fileCount` reads every
page of every part's children and counts the root `SHADOWS.md`, one file per part
and each part's listed files, restarting when the map's revision changes. That
count is Core's rule about what `create_structure` writes. The stage read must
carry it, so the walk can go. The person confirms the count they saw, so the
create command must name the map revision that count came from, and Core must
refuse to write if the map has moved since.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/projects/{id}/stage` | nothing | adds `structure_file_count: integer \| null`, the files `POST /api/projects/{id}/structure` would write for the map now, and `map_revision: integer`, the map revision that count was taken at; the count is `null` when the stage cannot create a structure |
| `POST /api/projects/{id}/structure` | adds `map_revision: integer`, the revision the person confirmed; it is part of the command's fingerprint | unchanged on success; 409 when the map's revision is no longer `map_revision`, and nothing is written |

## Errors

| When | The screen shows |
|---|---|
| The count is `null` | no count, and the confirm stays as the stage allows |
| The map changed after the person read the count | "The map changed; check the new count." The stage is read again, and the person confirms again |

## Backend answer

Hamoud, 2026-10-29: as asked. `Design` owns both (`crates/core-design`, contract
`crates/core-design/contract.yaml`): `Design::stage` returns the map revision and the count, from
one count `create_structure` also uses, so they cannot disagree. `create_structure` takes the
confirmed `map_revision`, refuses a moved map with 409 `REVISION_CONFLICT` and
`current_revision`, and writes nothing; the revision is part of the command's fingerprint.
Task: B-17.

## Frontend reply

Masada, 2026-10-29: accepted. F-48 shows the count, sends `map_revision`, and on
`REVISION_CONFLICT` reads the stage again and asks again.

## Ruling
