# A part has one folder and its layers; standards v2

- Status: accepted
- Asked by: Hamoud, 2026-11-06
- Screen: docs/backend/plans/2026-11-05-structure-and-depth.md, Slice 2 (B-63, B-64, B-65)

## Need

The Project map shows each part with its one folder and its layers, and the
standards view shows each part kind's presence and layer catalog. Every
response that carries a `PartContent` changes shape: `folders` is gone. The
standards response carries the v2 `BaseStandards`. Existing projects are
migrated: a part keeps its first old folder (or its title slug), starts with
no layers, and every pending map card is superseded so the Planner proposes
again.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| Every route that sends or returns a part (`PartContent`) | `{ folder: string, layers: PartLayer[] }` in place of `folders: string[]` | the same |
| `PartLayer` | `{ layer: string, place: string, not_used?: string \| null }` | the same |
| Standards read (`BaseStandards`) | | `{ version: 2, parts: PartKind[], rules: StandardRule[] }`; `contract_template` is removed |
| `PartKind` | | `{ name, owns, presence: "required" \| "waivable" \| "optional", layers: LayerSpec[] }` (replaces `MandatoryPart.waivable`) |
| `LayerSpec` | | `{ id, role, required: boolean, elements: string[], may_use: LayerRef[] }` |
| `LayerRef` | | `{ part_kind: string \| null, layer: string }`; `null` means the same part |

`place` is project-relative and lies inside the part's folder, for example
`backend/app/Domain`. It may hold one `{module}` marker, for example
`crates/core-{module}/src/model.rs`. A not-used layer has an empty `place`
and a written `not_used` reason.

## Errors

Map refusals keep `MapRefusal { rule, message }`; the screen shows the message.

| When | The screen shows |
|---|---|
| `one_folder`: a part has an empty folder | "part {title} has no folder. Give the part one folder." |
| `invalid_path`: a place holds `{module}` twice | the message, naming the part and layer |
| `missing_layer` (B-65) | a required layer is missing and not marked not used |
| `unknown_layer` (B-65) | a layer is not in its kind's catalog, or appears twice |
| `layer_outside_part` (B-65) | a place lies outside the part's folder |
| `layer_place_shared` (B-65) | two layers of one part can share a place |
| `layer_not_used_without_reason` (B-65) | `not_used` is blank, or set with a place |
| `layer_inside_child` (B-65) | a place can lie inside a child part's folder |

## Backend answer

Hamoud: as asked. `core-design` owns `PartContent` and `PartLayer`
(`crates/core-design/contract.yaml`); `core-standards` owns the catalog
(`crates/core-standards/contract.yaml`). Schema: `api/openapi.json`,
`web/src/api/schema.d.ts`.

## Frontend reply

Accepted by the inline controller, 2026-11-06: B-63/B-64 expose these shapes in
the generated API schema. F-90 consumes them through the existing design API.
The component shows Core's refusal messages; it adds no independent map policy.

## Ruling
