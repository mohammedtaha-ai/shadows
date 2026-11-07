# Principles, not layout

- Status: accepted
- Asked by: controller implementing both sides, 2026-11-07
- Screen: `docs/backend/specs/2026-11-05-structure-and-depth-design.md` §6.1

## Need

The map follows the chosen framework's layout. Shadows records parts and
layers without creating folders or starter files. Roadmap opens after map
approval, without a Structure step.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| Standards reads | Unchanged | Base v3; layer specs omit `elements`; rules include S10 |
| Part reads and writes, including MCP | Part content omits `files` | Part content omits `files`; no `InitialFile` schema |
| Stage reads | Unchanged | `Idea`, `Vision`, `Map`, `Roadmap` or `Plans`; no `structure_file_count` |
| `POST /api/projects/{id}/structure` | Removed | No registered operation |

## Errors

Existing error translation stays. Layer places may lie outside their part's
folder; malformed paths, missing layers and shared places still refuse.

## Backend answer

As specified: `core-standards` owns the catalog, `core-design` owns parts,
map validation and stage gates. HTTP and MCP translate the owner APIs.
Regenerate OpenAPI and the web schema after serialized changes.

## Frontend reply

Accepted by the controller for the approved Slice 3, implementing both sides
under Mohammed's instruction. Remove starter-file controls, Create structure
and the Structure step; show S10 and a stack-specific layer-place hint.
