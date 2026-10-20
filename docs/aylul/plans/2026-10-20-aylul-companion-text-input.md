# AYLUL Companion Text Input Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pressing the companion's Ready/status bar opens a small text input below the character that continues the selected agent's conversation, while main stays hidden.

**Architecture:** The companion window gets two modes that native code owns: compact non-focusable (closed) and focusable with a taller surface (open). The renderer sends only text, a command id and a native-issued context token. Native code checks the token and runs the web first-send's order of Core calls (create thread → bind → open session → default model → start turn). The existing read-only watcher remains the only source of the reply bubble.

**Tech Stack:** Tauri 2.11.5 (`WebviewWindow::set_focusable`), Rust, blocking reqwest 0.12.28, uuid v4/v5 (already dependencies), React in `apps/desktop/src`, `node --test`, Playwright. No new dependency.

**Spec:** [§8A of the desktop/companion design](../specs/2026-10-20-aylul-desktop-web-companion-design.md#8a-compact-text-input-below-the-character), with §7's gesture rules and §8's watcher rules.

## Global Constraints

- **Branch:** `codex/aylul-companion-text-input`. Commit once per task, stage files by name, never commit `.mcp.json`. No push and no PR until Mohammed says so.
- **No Core change:** no Rust change under `crates/`, and no new endpoint. `api/openapi.json` stays unchanged.
- **Companion source stays in `apps/desktop/src`:** it imports nothing from `web/`, and the renderer makes no `fetch`.
- **Size:** about 296 logical px wide; the input strip is 80 logical px high. Character and bubble keep their scale. No sidebar, transcript, header, voice or conversation picker.
- **Defaults and keys:**
  - Input is closed when the app starts.
  - Enter sends. Shift+Enter adds a newline. Enter while the IME is composing (`isComposing`) does nothing.
  - Escape or the status button closes the input. Clicking elsewhere does not.
- **Drafts:**
  - Draft text lives in renderer memory, keyed by the native `scope` token.
  - It survives close and hide. It is lost on quit or reload, and when the agent, the configuration or the bound conversation changes.
  - The one exception: the companion's own first send binding a new thread keeps the scope, so the input stays open after that send.
- **Window rules:**
  - Only an explicit open grants focus.
  - Presence, replies, show and recovery never open the input or take focus.
  - Hiding the companion closes the input. Showing it again does not reopen it.
- **Sends:**
  - One in-flight attempt at a time.
  - An unchanged Retry uses the same command id and resends the captured thread and turn body byte for byte. A timeout is "uncertain" and is never resent with a new id.
  - While a turn is running, sending is refused as busy and the text is kept. No queue, Stop or Send now.
- **Turn settings:**
  - A send uses the session's current model, mode and effort; native never picks a mode.
  - A new thread gets the agent's `default_model` when it differs, as the web draft does.
- **Secrets and errors:** no secret, key, URL or raw transport error crosses IPC or reaches a log. The renderer shows only fixed messages.
- **Builds:**
  - Native tests: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml <filter>`, over the existing default cache.
  - No `CARGO_TARGET_DIR`, no clean, no `--release`, `--profile` or `--target`.
  - Never run cargo and node/vitest at the same time.
- **During tasks:** each task runs only the tests it touches. The full gate runs once, in Task 5.
- **Before code:** read `docs/aylul/codebase/README.md` and the colocated `contract.yaml`. Update the changed contracts in the same commit.
- **File sizes:** a file that passes 300 lines names its one job in the commit message; a file at 500 lines splits. `windows/companion/mod.rs` (291 lines) does not grow: new window code goes in `input.rs`.

## Review Focus

1. **Arabic text and IME.** Arabic or emoji typed through an IME must not send on the Enter that commits the composition, and the text must reach Core byte-for-byte (Tasks 2, 4).
2. **Retry after a timeout on the first send.** It must make exactly one thread and one turn (Task 2).
3. **Agent or conversation changes while the input is open.**
   - Main switching to another conversation of the same agent, or to another agent, drops the old draft and closes the input.
   - A send already started still finishes on its captured thread and never binds into the new agent (Tasks 2, 3).
4. **Drags.** A drag that starts on the artwork and ends over the status button or input must not toggle input or open main (Task 4).
5. **Screen edges.** Opening at the bottom or right edge of a work area, or on a negative-origin monitor, keeps the whole open surface on screen. Closing returns the window to its exact earlier position (Task 1).

---

## File structure

| File | One job |
|---|---|
| `apps/desktop/src-tauri/src/windows/companion/input.rs` (new) | Open/closed window mode: bounds, focusability, revision |
| `apps/desktop/src-tauri/src/presence/send.rs` (new) | One explicit companion submission, from token check to started turn |
| `apps/desktop/src-tauri/src/presence/send_tests.rs` (new) | Fixture tests pinning the request order and bodies |
| `apps/desktop/src/features/companion/CompanionComposer.tsx` (new) | The input strip: keys, Send, status line |
| `apps/desktop/src/features/companion/composer-model.ts` (new) | Pure draft, attempt and error-message logic |
| `apps/desktop/tests/companion/composer.test.ts` (new) | Unit tests for `composer-model.ts` |
| `apps/desktop/tests/companion/composer.spec.ts` (new) | Browser fixture tests for keys and gesture isolation |

---

### Task 1: Native input mode and the focus probe

**Files:**
- Create: `apps/desktop/src-tauri/src/windows/companion/input.rs`
- Modify: `apps/desktop/src-tauri/src/windows/companion/{mod.rs,state.rs,contract.yaml}`
- Modify: `apps/desktop/src-tauri/src/commands/companion.rs`, `apps/desktop/src-tauri/src/lib.rs` (register the command), `apps/desktop/src-tauri/permissions/companion.toml`
- Modify: `apps/desktop/tests/windows-companion.ps1` and `apps/desktop/tests/companion-native.mjs` (probe cases)

**Interfaces:**
- Produces, in `state.rs`:
  - `pub struct ComposerView { pub open: bool, pub revision: u64, pub scope: Option<String>, pub context: Option<String>, pub agent: Option<String> }`, serialized camelCase.
  - `CompanionState` gets `pub composer: ComposerView`. Task 1 fills only `open` and `revision`. Task 3 fills the rest.
- Produces, in `input.rs`:
  - `pub fn open_bounds(closed: (i32, i32), scale: f64) -> (i32, i32, u32, u32)`: the open rectangle before fitting. Width `296 * scale`, height `(240 + 80) * scale`. `x` is `closed.0 - 28 * scale`, so the character stays centered. `y` is unchanged.
  - `pub fn closed_origin(open: (i32, i32), scale: f64) -> (i32, i32)`: the inverse of `open_bounds`. Use it to find the closed position after a drag while open.
  - `pub fn set_input(app: &tauri::AppHandle, open: bool) -> Result<CompanionState, String>`
- Produces, in `commands/companion.rs`: `#[tauri::command] companion_input(window, open: bool) -> Result<CompanionState, String>`, which only the `companion` window may call.

- [ ] **Step 1: Write the failing geometry and state tests** in `input.rs`:
  - `open_bounds_centres_the_character`: `open_bounds((1664, 784), 1.0) == (1636, 784, 296, 320)`. At `scale 1.5`: width 444, height 480, x `1664 - 42`.
  - `open_then_close_restores_exact_origin`: `closed_origin` of `open_bounds((x, y), s)`'s origin equals `(x, y)` for `(1664, 784, 1.0)`, `(-500, 300, 1.25)` and `(0, 0, 2.0)`.
  - `open_at_bottom_right_edge_fits_whole_rectangle`: `fit_companion` applied to `open_bounds((1664, 784), 1.0)` in area `(0, 0, 1920, 1040)` returns a rectangle fully inside the area.
  - In `state.rs`: `hide_closes_input_and_unhide_keeps_it_closed`. A model with `composer.open = true` gets `request_visible(false)`, then `true`. `composer.open` must be false both times, and `revision` must increase.
- [ ] **Step 2: Run** `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml companion::input`. Expected: FAIL, unresolved `open_bounds`.
- [ ] **Step 3: Implement `input.rs`, the model changes and the command.**
  - **Opening:**
    - Store the closed origin.
    - Compute the open bounds and pass them through `placement::fit_companion`.
    - Call `set_size` and `set_position`, then `set_focusable(true)`, then `set_focus()`.
  - **Closing:**
    - Call `set_focusable(false)` first.
    - Then restore the closed size (240×240 logical) at the stored origin. If the window was dragged while open, use `closed_origin` of the current position instead, passed through `fit_companion`.
  - **The `fit` handler** in `mod.rs` must not shrink an open window back to 240: pass the mode's size.
  - **Hiding:** `set_visible(false)` calls the close path before `no_activate::hide`.
  - **Threading:** every effect runs through `on_ui`. Each open or close bumps `composer.revision` and publishes.
  - **Fallback:** if `set_focusable` does not clear `WS_EX_NOACTIVATE` on Windows (Step 6 shows this), add the Win32 style toggle beside `show`/`hide` in `no_activate.rs`. Record which path was used in the commit message.
- [ ] **Step 4: Run** the Step 2 command, then `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml companion_callers_are_restricted`, with `companion_input` added to that test's list as companion-only. Expected: PASS.
- [ ] **Step 5: Build the program under test,** one command after the other, over the existing cache:
  1. `npm --prefix apps/desktop run build:desktop`
  2. `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml`

  The debug build embeds `apps/desktop/dist` (`frontendDist`, no `devUrl`), so a probe runs the code it names. Expected: `src-tauri/target/debug/nylra-desktop.exe` is newer than this task's last source edit.
- [ ] **Step 6: Window-focus probe only.** No input element exists yet, so this step tests focus and window style, not typing.
  - Extend `companion-native.mjs`, started by `windows-companion.ps1`, using its existing CDP `invoke` and focus-witness form:
    1. `invoke(companion, 'companion_input', {open: true})`: the foreground HWND becomes the companion's, `WS_EX_NOACTIVATE` is cleared, the window is 296×320 logical, and main stays hidden.
    2. `invoke(companion, 'companion_input', {open: false})`: `WS_EX_NOACTIVATE` is set again, the size and position are back to the values before step 1, the witness form can take the foreground back, and main is still hidden.
    3. Open, then hide, then show: `composer.open` is false, and the foreground does not change on show.
  - Add `companion_input` to the probe's must-reject-main list.

  Expected: every assertion passes. **If any fails, stop:** report it and amend spec §8A before Task 2.
- [ ] **Step 7: Update `windows/companion/contract.yaml`** with `set_input` and the obligations "only explicit open grants focus" and "hide closes input; unhide does not reopen". Then commit:
  `git add <files above> && git commit -m "feat(companion): native open/closed input mode with explicit focus"`

### Task 2: Native send sequence

**Files:**
- Create: `apps/desktop/src-tauri/src/presence/send.rs`, `apps/desktop/src-tauri/src/presence/send_tests.rs`
- Modify: `apps/desktop/src-tauri/src/presence/{mod.rs,binding.rs,contract.yaml}`

**Interfaces:**
- Consumes:
  - `Presence::request(Some(generation)) -> RequestContext`
  - `Presence::commit` (made `pub(super)`)
  - `binding::{workspace, live_agent, target}`
- Produces:
  - In `binding.rs`: `Agent` gets `#[serde(default)] pub default_model: Option<String>`.
  - In `send.rs`:
    - `#[derive(Deserialize)] #[serde(rename_all = "camelCase", deny_unknown_fields)] pub struct Submission { pub text: String, pub command_id: String, pub context: String }`
    - `#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct Sent { pub thread_id: String, pub operation_id: String }`
    - `struct Captured { command_id: String, scope: String, thread_id: String, body: serde_json::Value }`: the exact turn request of the one pending attempt. It lives in `Presence`'s `Model` as `Option<Captured>` and is never written to disk.
    - `impl Presence { pub fn context(&self) -> (Option<String>, Option<String>) }`, returning `(scope, context)`. Native mints both tokens as uuid v4:
      - `scope` changes when the agent, the configuration or the target thread changes. The exception is the target that `send` itself binds on a first send (`None` → `Some(T)`).
      - `context` changes whenever the binding generation changes.
      - Both are `None` when no agent is selected.
    - `impl Presence { pub fn send(&self, submission: Submission) -> Result<Sent, &'static str> }`
  - Error codes returned to IPC:
    - `companion_send_invalid`: empty text, more than 32 768 bytes, or a command id that is not a UUID.
    - `companion_send_stale`: the token does not match.
    - `companion_agent_unavailable`
    - `companion_thread_invalid`
    - `companion_send_busy`: `THREAD_BUSY`.
    - `companion_send_setup`: `AYLUL_KEY_MISSING`, `AYLUL_NOT_CONFIGURED`, `AYLUL_PROFILE_NOT_FOUND`, `AYLUL_PROVIDER_NOT_FOUND`, `RUNTIME_NOT_CONFIGURED`.
    - `companion_send_settings`: `MODE_NOT_ALLOWED`, `SETTING_NOT_OFFERED`.
    - `companion_send_uncertain`: timeout or a dropped connection after the request was written.
    - `companion_core_unavailable`
    - `companion_send_failed`: anything else.

**The sequence `send` runs:**
1. Validate the text and the command id.
2. **Retry of a captured attempt:**
   - When `Captured.command_id == command_id`:
     - The text must equal the captured prompt, else `companion_send_invalid`.
     - The current `scope` must equal `Captured.scope`, else `companion_send_stale`. The context token is not checked, because the first send's own bind changed it.
   - Skip steps 3–6, and POST `Captured.body` unchanged to `Captured.thread_id` (step 7).
   - Otherwise: check that `context` equals the current token, and capture `request(Some(generation))`. In the same lock, record the `admitted_scope`: the scope at the moment this send is accepted. Every later step uses `admitted_scope`, never the scope current at that time.
3. `workspace` + `live_agent`.
4. Choose the thread:
   - **With a target:** re-validate it with `binding::target`.
   - **Without one:**
     - POST `/api/projects/{project}/threads` with `{"command_id": create_id, "title": "New conversation", "harness": "aylul", "profile": agent}`. The create id is `Uuid::new_v5(&NAMESPACE, format!("companion-thread:{command_id}"))`, using a fixed `NAMESPACE` constant in `send.rs`.
     - Then `commit(request, |view| view.target = Some(target))`. If the commit fails, return `companion_send_stale`.
5. POST `/api/threads/{id}/session` and read `SessionChoices.current`.
6. **New thread only:** if the agent's `default_model` is set and differs from the current model, PUT `/api/threads/{id}/session/model` with `{"model": ...}` and use the returned `current`.
7. Build the turn body `{"command_id", "prompt": text, "model", "mode", "effort", "focus": null, "client_tab": null}`. Store it as `Captured` with the thread id and `admitted_scope`, then POST it to `/api/threads/{id}/turns`.
   - The store happens only if the current scope still equals `admitted_scope` and the `Captured` slot is empty or holds this `command_id`.
   - Otherwise nothing is stored. The turn still posts to its captured thread, since an admitted request is never redirected, but this attempt can no longer be retried.
8. **What happens to `Captured`:**
   - Cleared on success, and on any definitive refusal (`setup`, `settings`, `thread_invalid`, `failed`).
   - Kept on `uncertain` and `busy`, because Core did not admit the command, or may have.
   - Cleared when the scope changes.
   - A result, on time or late, changes `Captured` only while the slot holds that result's own `command_id`. An older send that finishes late never replaces or clears a newer scope's attempt.

   A retry therefore sends the same command id with byte-identical body to the same thread, so Core replays the command instead of answering `COMMAND_CONFLICT`.

**HTTP rules:**
- Same client settings as `binding::read`: `no_proxy`, no redirects, 1 MiB cap. Timeouts: 10 s for create, 30 s for session and turn.
- Errors are matched on `ErrorBody.code` only; `message` is never logged or returned.
- After a successful commit, the watcher follows through the existing notifier. Step 7 does not wait on it.

- [ ] **Step 1: Write the failing fixture tests** in `send_tests.rs`. Reuse the `Api` pattern from `binding_tests.rs`, extended so it records each request's method, path and JSON body, and answers from a script:
  - `first_send_creates_binds_opens_and_starts_in_order`:
    - Called with no target.
    - The recorded paths are exactly `POST /api/projects/P/threads`, `POST /api/threads/T/session`, `PUT /api/threads/T/session/model`, `POST /api/threads/T/turns`, among the GET reads.
    - The create body has `harness: "aylul"`, `profile: "agent-a"`, `title: "New conversation"`.
    - The turn body has `prompt` byte-equal to `"مرحبا 👋\nثاني سطر"`, plus the session's mode and effort.
    - `presence.view().target.thread_id == T`.
  - `bound_send_reuses_target_and_keeps_session_settings`: with a target, there is no create and no model PUT, and the turn carries the session's `current` unchanged.
  - `uncertain_create_retry_makes_one_thread`:
    - The create POST stalls past a 200 ms test timeout, giving `companion_send_uncertain`.
    - The retry with the same `command_id` sends the same create `command_id`, and the fixture returns the same thread for it.
    - Exactly one distinct thread id and one turn are ever recorded.
  - `uncertain_turn_retry_sends_the_captured_request`:
    - The fixture models Core idempotency: the same turn `command_id` with an identical body returns the first `operation_id`, and with a different body returns 409 `COMMAND_CONFLICT`.
    - The first turn POST stalls past the test timeout, giving `companion_send_uncertain`.
    - Between the attempts, the fixture's session answers a different `mode` and `model`, and main toggles previews. That bumps the context token but keeps the conversation and the scope.
    - The retry records no session or model call, and its turn body is byte-equal to the first.
    - It returns `Ok(Sent)` with the first `operation_id`.
    - A retry with the same `command_id` but changed text gives `companion_send_invalid` and records no request.
  - `retry_after_main_switches_conversation_is_refused`: after an uncertain turn on T1, main retargets to T2. A retry with the same `command_id` gives `companion_send_stale`, records no request, and the `Captured` slot is empty.
  - `late_old_send_cannot_replace_newer_attempt`:
    - Send A is admitted under scope S1, and its session POST stalls.
    - Main retargets (scope S2). Send B is admitted under S2, and its turn POST times out: `Captured` holds B.
    - A's session then answers, and A's turn POST times out.
    - `Captured` still holds B with scope S2, and A's turn was posted to T1, never to B's thread.
  - `main_switching_conversation_changes_scope`:
    - When main retargets agent A from thread T1 to T2, `context().0` changes.
    - When `send` binds a first thread, `context().0` stays the same.
  - `stale_context_is_refused_before_any_write`: after `select` changes the agent, an old `context` gives `companion_send_stale`, and the fixture recorded no POST.
  - `binding_change_mid_send_completes_on_captured_thread`:
    - Main retargets after the session POST.
    - The turn still posts to the captured `T`, and the binding stays on main's new target.
  - `error_codes_map_to_fixed_values`:
    - `THREAD_BUSY` gives `companion_send_busy`.
    - `AYLUL_KEY_MISSING` gives `companion_send_setup`.
    - `MODE_NOT_ALLOWED` gives `companion_send_settings`.
    - A 500 response gives `companion_send_failed`.
    - The response's `message` string never appears in any `Err` value.
  - `removed_target_is_reported_not_replaced`: when the bound thread has `removed_at` set, the send gives `companion_thread_invalid`. No create POST is made, and the target is unchanged.
  - `invalid_submissions_send_nothing`: empty, whitespace-only and 32 769-byte text, and a non-UUID id. Each gives `companion_send_invalid` with no request at all.
- [ ] **Step 2: Run** `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence::send`. Expected: FAIL, unresolved `send`.
- [ ] **Step 3: Implement `send.rs`** as described above.
  - Store the two tokens in `Presence`'s `Model`. Reissue them inside `commit`, `select`, `previews` and `configuration_changed`.
  - Make the HTTP timeouts a field that a test can set.
- [ ] **Step 4: Run** the Step 2 command, then `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence`, so the existing binding and watcher tests stay green. Expected: PASS.
- [ ] **Step 5: Update `presence/contract.yaml`.**
  - Replace the `responsibility` wording "without mutating Core" with: "Observe the bound conversation; mutate Core only for an explicit companion submission."
  - Add `send` and `context` to `public_api`.
  - Add obligations naming the twelve tests above.
  - Commit: `feat(companion): native send sequence for the companion input`

### Task 3: IPC command and the composer view

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/{companion.rs,contract.yaml}`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/permissions/companion.toml`
- Modify: `apps/desktop/src-tauri/src/windows/companion/input.rs` (fill `scope`, `context` and `agent`)
- Modify: `apps/desktop/src/platform/{companion.ts,companion-state.ts}`, `apps/desktop/tests/companion/native-state.test.ts`, `apps/desktop/tests/native-acl.test.mjs`

**Interfaces:**
- Produces:
  - `#[tauri::command] companion_send(window, submission: Submission) -> Result<Sent, String>`. Companion window only. It runs `Presence::send` in `spawn_blocking`.
  - TS: `export type ComposerView = {open: boolean; revision: number; scope: string | null; context: string | null; agent: string | null}`. `CompanionState` gets `composer: ComposerView`, validated in `acceptState`: strings at most 128 characters, `revision` a safe integer.
  - TS: `export function setCompanionInput(open: boolean): Promise<CompanionState>` and `export function sendFromCompanion(text: string, commandId: string, context: string): Promise<{threadId: string; operationId: string}>`.

- [ ] **Step 1: Write the failing tests.**
  - In `native-state.test.ts`: `composer view is validated and never optional`. A state without `composer`, with `open: 'yes'`, or with a 129-character `scope` throws. A valid state round-trips.
  - In `native-acl.test.mjs`: assert that `companion-actions` allows exactly the earlier five plus `companion_input` and `companion_send`, and that `main-actions` does not contain them.
  - In `commands/companion.rs`: add both names to the restriction test as companion-only.
- [ ] **Step 2: Run** `npm --prefix apps/desktop run test:companion:unit` and `node --test apps/desktop/tests/native-acl.test.mjs`. Expected: FAIL.
- [ ] **Step 3: Implement the changes.**
  - The Presence notifier in `lib.rs` also refreshes `composer.{scope, context, agent}` and publishes.
  - When `scope` changes while the input is open, call `set_input(false)`.
- [ ] **Step 4: Run** the Step 2 commands, then `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml companion`. Run them one after another. Expected: PASS.
- [ ] **Step 5: Update `commands/contract.yaml` and the platform `contract.yaml`,** then commit: `feat(companion): restricted input and send IPC`

### Task 4: The input strip and status toggle

**Files:**
- Create: `apps/desktop/src/features/companion/{CompanionComposer.tsx,composer-model.ts}`, `apps/desktop/tests/companion/{composer.test.ts,composer.spec.ts}`
- Modify: `apps/desktop/src/features/companion/{DesktopCompanion.tsx,desktop.css,desktop-gesture.ts,contract.yaml}`, `apps/desktop/package.json` (add `composer.test.ts` to `test:companion:unit`), `apps/desktop/tests/companion/{fixture.tsx,fakes.ts}`

**Interfaces:**
- Consumes: `setCompanionInput` and `sendFromCompanion` from Task 3, and `openMainFromCompanion`.
- Produces, in `composer-model.ts`:
  - `createDrafts(): {read(scope: string): string; write(scope: string, text: string): void; drop(keep: string | null): void}`. `drop` removes every scope except `keep`.
  - `nextAttempt(previous: {scope: string; text: string; commandId: string} | null, scope: string, text: string, newId: () => string): {scope: string; text: string; commandId: string}`. It reuses `previous.commandId` exactly when both scope and text are unchanged.
  - `submitKey(event: {key: string; shiftKey: boolean; isComposing: boolean}): 'send' | 'newline' | 'close' | null`
  - `errorMessage(code: string): string`, with these fixed messages:
    - `companion_send_busy`: "AYLUL is still answering. Your text is kept."
    - `companion_send_uncertain`: "Not sure it was sent. Retry sends the same message."
    - `companion_send_stale`: "AYLUL changed. Check the agent and send again."
    - `companion_send_setup`: "Finish this agent's setup in Settings."
    - `companion_send_settings`: "This conversation's mode is not allowed here. Open it in main."
    - `companion_thread_invalid`: "This conversation was removed. Choose it again in Settings."
    - `companion_core_unavailable`: "Core unavailable. Retry local Core in Settings."
    - Anything else: "Unable to send. Retry."

- [ ] **Step 1: Write the failing unit tests** in `composer.test.ts`:
  - Drafts: survive `drop(sameScope)`, vanish on `drop(otherScope)`.
  - `nextAttempt`: keeps the id for the same scope and text, and makes a new id when either changes.
  - `submitKey`: Enter gives send; Shift+Enter gives newline; Enter with `isComposing` gives null; Escape gives close.
  - `errorMessage`: an unknown code gives the generic message, and no output contains the input code.
- [ ] **Step 2: Write the failing browser tests** in `composer.spec.ts`, against the existing fixture and fakes:
  1. **Status button:** it is a `button` with `aria-expanded`. A real Playwright `click()`, Enter and Space each toggle the input. None of them calls `openMainFromCompanion` or `dragCompanion`.
  1a. **Real clicks reach the strip:** a Playwright `click()` on Send sends once. A `click()` into the textarea places the caret there. Neither calls `openMainFromCompanion`.
  1b. **Artwork is unchanged:** a `click()` on the artwork still calls `openMainFromCompanion` once.
  2. **Artwork drag:** a pointer drag that starts on the artwork and ends on the status button neither toggles input nor opens main.
  3. **Open state:** with the input open, the textarea has focus. Shift+Enter inserts a newline. Enter during `compositionstart`…`compositionend` does not send.
  4. **Double send:** a double Enter sends once, and Send is disabled while sending. On `companion_send_busy` the text stays and the busy message shows.
  5. **Retry after uncertain:** Retry after `companion_send_uncertain` calls `sendFromCompanion` with the same `commandId`.
  6. **No agent:** with `context: null`, the strip shows a "Choose an agent" button that calls `openMainFromCompanion`, and no textarea.
  7. **Draft lifetime:** close, then reopen, and the draft is still there. A state with a new `scope` (main switched between two conversations of the same agent) empties the draft and drops the pending attempt, so the next Send uses a new `commandId`.
  8. **Pause:** with `paused: true`, typing and Send still work.
- [ ] **Step 3: Run** `npm --prefix apps/desktop run test:companion:unit` and `npx --prefix apps/desktop playwright test tests/companion/composer.spec.ts`. Expected: FAIL.
- [ ] **Step 4: Implement.**
  - **Status bar:** replace the `aylul-live-status` paragraph with a `<button aria-expanded>` that keeps `role="status"` text inside a child `span`.
  - **Event isolation:**
    - Today the root `.aylul-desktop` element carries the pointer handlers, the `onClickCapture` that calls `preventDefault`/`stopPropagation`, and the Enter/Space open-main `onKeyDown`. A capture-phase handler on an ancestor runs before any child, so `stopPropagation` on the button cannot escape it.
    - Move all of these handlers, and `setPointerCapture`, from the root onto a new `.aylul-artwork` wrapper around `CompanionCanvas` only.
    - The bubble, status button and strip become siblings of that wrapper, not its descendants, so no gesture handler sees their events.
  - **Text:** the textarea uses `dir="auto"`. Read `event.nativeEvent.isComposing` for `submitKey`.
  - **Successful send:** clear the draft and keep the input open. The reply arrives through the existing bubble.
  - **Agent caption:** show `composer.agent` as plain text.
  - **Layout:** the open layout puts the strip below the status, inside the 296×320 surface. The closed layout is unchanged.
- [ ] **Step 5: Run** the Step 3 commands, then `npm --prefix apps/desktop run test:companion:browser`, so the existing gesture, presence and visual specs stay green. Expected: PASS.
- [ ] **Step 6: Update `features/companion/contract.yaml`** with the composer, the gesture isolation and the draft lifetime. Commit: `feat(companion): Ready toggles a compact text input under the character`

### Task 5: Docs, full gate, native acceptance

**Files:**
- Modify: `docs/aylul/codebase/README.md` (rows for `input.rs`, `send.rs`, the composer), `docs/aylul/ui/stages.md`, `docs/aylul/specs/2026-10-20-aylul-desktop-web-companion-design.md` (Status line: implemented through `<sha>`)
- Create: `docs/evidence/aylul/2026-10-20-companion-text-input.md`

- [ ] **Step 1: Desktop gate,** in the background, in this order, each command after the previous one finishes:
  1. `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`
  2. `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`
  3. `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
  4. `npm --prefix apps/desktop test`, then `run test:companion:unit`, `run test:companion:browser`, `run test:companion:production`
  5. `npm --prefix apps/desktop run build:desktop`

  Expected: every one exits 0. Record the test counts.
- [ ] **Step 2: Root gate,** steps 1–5 of `CLAUDE.md`. Core is unchanged, so a pass here proves the branch did not touch it. Also check that `git diff --exit-code main -- crates api web` is empty.
- [ ] **Step 3: Rebuild the program under test** with Task 1 Step 5's two commands, run one after the other once the gate has finished. Expected: a fresh `nylra-desktop.exe`.
- [ ] **Step 4: Native acceptance** on the actual program and the existing database, run with the extended `windows-companion.ps1`. It reruns Task 1's window-focus probe, plus:
  - **Typing, now that the input exists:**
    - A status-button click through CDP opens the input.
    - SendKeys types `ab`, and the textarea's value reads `ab`.
    - Escape closes the input, and the foreground returns to the witness form.
  - `companion_send` is in the must-reject-main list.
  - Open at the right/bottom edge.
  - Hide to tray, then show: the input stays closed.
  - One real send from the input to a bound agent; main stays hidden.

  Write the outcome to the evidence file. Separate automated fixture results from what only Mohammed's real-provider journey can show.
- [ ] **Step 5: Update the code map, the stages and the spec Status line,** then commit: `docs(aylul): companion text input delivered`
