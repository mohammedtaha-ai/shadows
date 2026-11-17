# Executor choices: CLI, model and effort per task

- Status: asked
- Asked by: Masada, 2026-11-18
- Screen: the task page (`web/src/app/tasks/`), the draft conversation's composer
  (`web/src/app/conversation/draft.tsx`); backend plan
  [executor choices](../../backend/plans/2026-11-18-executor-choices.md), section "The web API"
  (B-157, B-158). Web tasks follow once this is accepted.

## Need

On a task's page Mohammed sees which CLI, model and effort the task will run on, and who chose
it (the Planner or him). He changes any of the three from lists the CLI itself offers, and Start
runs on what is saved. A task with no choice shows why Start is closed. Each attempt's Run tab
shows the CLI, model and effort it really ran on. In a new conversation, before the first
message, the composer lists the CLI's real models and efforts instead of "Set on send", starting
on the remembered ones.

The web holds no list of models or efforts and no rule on which effort a model needs: it draws
the catalog Core gives and shows Core's refusal.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/harnesses` | — | `HarnessInfo[]` gains `executor: boolean`. The task page's CLI list is the entries with `executor: true`; an entry with `available: false` is shown disabled with its `reason`. |
| `GET /api/harnesses/{kind}/catalog` | — | `HarnessCatalog { harness: string, models: CatalogModel[] }`, `CatalogModel { id, label, description: string \| null, efforts: Choice[] }`, models in the CLI's order. `efforts: []` means the model offers none. |
| `GET …/versions/{version}/tasks/{task}` | — | `TaskRun.executor: ExecutorChoice \| null`, **replacing** `suggested_model`. |
| `PUT …/versions/{version}/tasks/{task}/executor` | `{ command_id: string, harness: string, model: string, effort: string \| null }` | `ExecutorChoice { harness, model, effort: string \| null, chosen_by: "person" \| "planner", chosen_at }` |
| `POST …/versions/{version}/tasks/{task}/attempts` | `{ command_id: string }` (no `model`) | `Attempt`, which gains `harness: string` and `effort: string \| null` |
| Ready groups (`ReadyTask`) | — | `ReadyTask.executor: ExecutorChoice \| null`, replacing `suggested_model` |
| Project stream | — | `TaskExecutorChosen { project_id, plan_id, task }` |

Asked, besides the plan's table:
- `ExecutorChoice`, `ExecutorInput`, `Chooser`, `HarnessCatalog` and `CatalogModel` named in
  `components.schemas` of `api/openapi.json`, so the generated types name them.
- `no_executor_choice` and `executor_not_offered` listed in the `RunRefusal` codes the schema
  gives, as the other Runs refusals are.

How the web uses them:
- The effort list is the selected model's `efforts`; when it is `[]` the effort control is
  hidden and the save sends `effort: null`. Changing the model picks no effort by itself: the save
  waits for one, and Core's `effort: required` stays the rule.
- On `TaskExecutorChosen` the web refetches the open task pages and ready groups (by key prefix);
  it needs no workflow id or version in the event.
- The draft composer reads `…/catalog` for the picked CLI and starts on its `remembered` model and
  effort. When the catalog fails or the remembered model is not in it, it falls back to today's
  "Set on send". The first send carries `model` and `effort` as today; no new route.

## Errors

| When | The screen shows |
|---|---|
| `GET …/catalog` answers `HARNESS_UNAVAILABLE` | That CLI disabled in the list, with Core's message |
| `GET …/catalog` answers `HARNESS_START_FAILED` | "Could not read {label}'s models." and a Retry (the next call probes again) |
| The first catalog read takes several seconds | The lists show loading; the rest of the page stays usable |
| `PUT …/executor` answers 409 `executor_not_offered` | Core's message beside the control; the saved choice stays shown |
| `PUT …/executor` answers `SUPERSEDED`, `ARCHIVED` or `NOT_FOUND` | Core's message, and the page refetches the task |
| `TaskRun.executor` is `null` | "No executor chosen yet" and Start closed with `start`'s reason |
| Start answers 409 `no_executor_choice` (a race with the page) | Core's reason, and the task refetches |
| A saved model the CLI no longer offers | Shown as saved; the attempt then fails with Core's reason, as any failed attempt |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why.>

## Frontend reply

<Masada, once answered.>

## Ruling

<The teacher, only when disputed.>
