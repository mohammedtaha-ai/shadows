# Structure and depth on a real provider (B-165)

Date: 2026-11-18. Branch `feature/code-settings-servers-row` at `f497c91c`; daemon from that
build on `shadows-dev`, web on `localhost:5173`. Project `accept-laravel` (new, empty folder
`F:\shadows_projects_testing\accept-laravel`). Planner on Claude Code, Sonnet 5.5, medium
effort, Chat Mode, mode Auto. The person's clicks were made by the controller session in the
browser pane; Mohammed confirms this record.

Each refusal below is the tool answer the Planner received, as shown in its conversation; the
daemon's debug log does not log tool refusals.

| # | Step (spec §11) | Observed |
|---|---|---|
| 1 | A map before the vision is approved | Refused: `STAGE_BLOCKED: vision incomplete`. The Planner proposed five vision cards (PU1, U1, G1, B1, TD1) instead; the person approved them. |
| 2 | A Laravel map: the database a child of the backend, its connection at `backend/config/database.php` | Proposed as asked: Backend in `backend` (domain, application, interface, infrastructure, composition, each placed in Laravel's own layout); Database in `backend/database`, child of the backend, `migrations` at `backend/database/migrations`, `connection` at `backend/config/database.php`, `schema` not used with a reason; `api` and `frontend` waived with reasons. |
| 3 | A not-used layer that also carries a place | Refused on the Planner's first map: a layer that is not used must carry a reason and no place. The Planner fixed the schema layer and the map went through. |
| 4 | The map approved; Roadmap opens with no Structure step | The person approved "map and waivers" (waivers need the person's button). The stage line went from `map` to `roadmap` directly. |
| 5 | Roadmap, then a plan whose first task runs `composer create-project` in `backend/` | Roadmap with one outcome and four acceptance items, approved by the person. Plan "Setup", v1: T1 "Scaffold Laravel backend and Notes layers" (runs `composer create-project laravel/laravel` into `backend/`), T2 "Notes migration and model" needs T1; two phases each. No task was run. |
| 6 | One operation per task per edit | A combined edit was refused: "two operations on T1 in one edit; two operations on T2 in one edit". The Planner split it. |
| 7 | A cycle is refused, naming the whole path | T1-needs-T2 refused: `WORKFLOW_VALIDATION_FAILED: these links form a cycle: T1, T2`. A read-back showed the plan unchanged (only T2 needs T1, revision 2). |
| 8 | The result survives a daemon restart | After stopping and starting the daemon: the conversation showed `Stage: plans`, the seven approvals (five vision cards, the map, the roadmap) and the Setup plan as before. |
| 9 | Show phases | Each task became a frame with its phases as `T1 · P1`, `T1 · P2`, `T2 · P1`, `T2 · P2`; the URL kept `?phases=true`. The plan header showed "2 tasks · 0 done" (F-127). |
| 10 | A fresh agent reads `task_context` without the old chat | In a new conversation, the first call on the Draft plan was refused: `INVALID_COMMAND: task_context needs an approved plan version`. After the person approved the plan, the same call named T1's part (Backend, from its write path `backend/**`), the five layers with their places, the Database child's layers, and rules S1, S3, S5 to S10, with S4 on the Database part. |

Not run: the FastAPI example (its own package layout approved without a folder refusal) and
the "two layers at one place" refusal. Both are covered by the store's tests, not by this run.
