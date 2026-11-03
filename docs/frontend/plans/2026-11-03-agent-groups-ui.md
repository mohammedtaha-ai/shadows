# Agent Groups UI Plan (messaging slice 1, client side)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** draft for review, 2026-11-03. B-40 to B-45 are built on this branch
(`docs/agent-messaging-design`). Their routes and schemas in `api/openapi.json` match this plan:
`Group { id, name, members[{ slug, name, thread_id }], needs_you, created_at }`, `Message { id,
group_id, seq, speaker, to, kind, body, created_at }`, where `speaker` is `person`,
`agent { slug, name }` or `shadows`. Its tasks go into `docs/frontend/stages.md` and
`docs/aylul/ui/stages.md` after review.

**Goal:** In the main window, Mohammed creates a group of AYLUL agents, opens it, writes with
`@` completion, and reads the answers live. On the desktop, AYLUL tells him when a group needs him.

**Architecture:**
- **The web half** (`web/`, the main window) gets a groups list in the AYLUL workspace sidebar, a
  "new group" dialog, and a group page at `/aylul/groups/$groupId`.
- The page refreshes from the AYLUL project's stream: a `durable` frame of kind `MessageAdded`
  invalidates that group's messages. It uses the refresh map that exists today; no new stream.
- **The desktop half** (`apps/desktop/`) adds three things:
  - a "needs you" count to the panel's status pill;
  - an alert when a message addresses Mohammed;
  - a `MainDestination::Group` that opens the group page.
- All three go through the native Core client, as the pill does today.

**Spec:** `docs/backend/specs/2026-11-03-agent-messaging-design.md` §6.6, §6.7.
**Backend plan:** `docs/backend/plans/2026-11-03-agent-groups.md` (B-45: routes and stream;
B-46: `Group.needs_you_since`, needed by U-10).

## Global Constraints

- One job per file; no `helpers`/`utils`. Each changed module updates its `contract.yaml` in
  the same commit.
- Types come from `web/src/api/schema.d.ts`, generated from `api/openapi.json`; no hand-written
  copies of `Group` or `Message`.
- Every mutation sends a `command_id` from `web/src/api/command-id.ts`.
- Text may be Arabic or English: message bodies use the existing `bidi.ts`.
- The renderer of the desktop never calls HTTP; the native side does (agents spec §7.6).

## Review Focus

1. **Two answers arrive while the page is scrolled up.** The view keeps its place and shows a
   "new messages" mark instead of jumping.
2. **A group whose member was deleted.** Past lines keep the name; the composer does not offer
   the deleted member in `@` completion.
3. **The stream drops and reconnects.** Messages written meanwhile appear once, in `seq` order.
4. **A `Notice` from Shadows** (a failed member, the limit reached) looks different from chat, so
   Mohammed does not read it as an agent's words.
5. **A member asks permission inside a group.** The request shows in the group and is answered
   there; the pill counts it once.
6. **A send whose answer is lost.** Core saved the message, the browser never heard. Sending
   again replays the first send: one message, one turn.

---

## Web (Masada's side: `web/`)

### Task F-81: Groups API module

**Files:**
- Create: `web/src/api/groups.ts`, `web/src/api/groups.test.ts`
- Modify: `web/src/api/contract.yaml`

**Interfaces:**
- Produces: `groupKeys`, `groupsQuery`, `groupQuery(id)`, `groupMessagesQuery(id)`,
  `createGroup({ name, members })`, `sendToGroup(id, text)`.
- Produces: the types `Group`, `Message` and `Speaker` from `components['schemas']`.
- `createGroup` and `sendToGroup` take the `command_id` from their caller. The caller keeps an
  `Attempt` (`attemptFor` in `web/src/api/command-id.ts`) until the request succeeds or its
  content changes, so a retry after a lost answer reuses the key and Core replays it (F-83's
  dialog and F-84's composer do this).

- [ ] Tests `sends_the_callers_command_id`, `reads_messages_after_a_seq`.
- [ ] Run `npm test -- groups` (in `web/`); expect FAIL, then implement, then PASS.
- [ ] Commit `feat(web): groups API`.

### Task F-82: The stream refreshes a group

**Files:**
- Modify: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`,
  `web/src/stream/contract.yaml`

**Interfaces:**
- `classifyRefresh` maps `MessageAdded` (payload `{group_id, seq}`) to
  `{ kind: 'group', groupId }` and `{ kind: 'groups' }`.
- `dispatchRefresh` invalidates, for `group`, both `groupMessagesQuery(groupId)` and
  `groupQuery(groupId)` (the open group's own `needs_you`); for `groups`, `groupsQuery`.
- **Who holds the AYLUL project's stream:** F-83's group list calls
  `useProjectEvents(agents.project_id)` (`agentsQuery` answers `project_id`). The list shows
  whenever the AYLUL sidebar does, so the badge stays live outside the group page. The group
  page holds it too; holders share one subscription.

- [ ] Test `a_message_added_refreshes_its_group_its_log_and_the_list`.
- [ ] Run; FAIL, implement, PASS.
- [ ] Commit `feat(web): a new group message refreshes its views`.

### Task F-83: Groups in the AYLUL sidebar, and the new-group dialog

**Files:**
- Create: `web/src/app/sidebar/group-list.tsx`, `web/src/app/groups/new-group-dialog.tsx`,
  `web/src/app/groups/new-group-dialog.test.tsx`, `web/src/app/groups/contract.yaml`,
  `web/src/app/groups/index.ts`
- Modify: the AYLUL workspace sidebar that holds `agent-list.tsx`; `web/src/app/sidebar/contract.yaml`

**Interfaces:**
- **The list:** above the agents (decided). Each group shows its name, its members' colour dots, and a "needs you" dot.
- **The list's stream:** it holds the AYLUL project's events (F-82).
- **The dialog:** a name and up to 6 live agents. It refuses an empty name or no member, and
  disables the seventh pick. On success, it invalidates `groupsQuery` (creating a group writes
  no `MessageAdded`, so the stream will not do it), then navigates to the group's page.
- **The dialog's retry:** it keeps one `Attempt` for `{ name, members }`; the same content after
  a failed or lost answer reuses its `command_id`.

- [ ] Tests `the_seventh_agent_cannot_be_picked`, `creating_a_group_opens_it`,
  `a_new_empty_group_appears_in_the_list_at_once`,
  `a_retry_after_a_lost_answer_reuses_the_command_id`.
- [ ] Run; FAIL, implement, PASS.
- [ ] Commit `feat(web): groups in the sidebar and the new-group dialog`.

### Task F-84: The group page

**Files:**
- Create: `web/src/app/groups/group-page.tsx`, `group-log.tsx`, `group-composer.tsx`,
  `mention-menu.tsx`, and their tests
- Modify: `web/src/router.tsx` (route `/aylul/groups/$groupId`)

**Interfaces:**
- **The log:**
  - Messages in `seq` order. An agent's line shows `speaker.name` (Core stores it, so a deleted
    agent keeps its name) in that agent's colour (`colorOf`), or a neutral colour when the agent
    is gone.
  - Mohammed's line is styled as his. A `Notice` is a quiet system line, not a speech bubble.
  - Lines addressed to Mohammed are marked.
  - On a new message, the log follows only if it was already at the bottom; otherwise it shows a
    "new messages" mark.
- **"Thinking":**
  - A member shows "thinking" while its hidden thread has a running turn.
  - It is read from each `members[].thread_id` through `GET /api/threads/{id}/operations` (the
    query the conversation page uses): a member is thinking while its latest operation is not
    terminal. No new route.
  - **It stays live:** the project stream carries `MessageAdded` but not a member thread's
    operation events. The page holds `holdThreadRefresh(client, thread_id, { projectId })`
    (`web/src/stream/refresh.ts`) for each member thread (6 at most) while it is open, so
    the operations query refreshes when a turn starts and when it ends.
- **The composer:**
  - Typing `@` opens the member menu: live members only. Mohammed is the writer, so
    `@mohammed` is not offered.
  - Enter sends; the field clears only after `sendToGroup` answers.
  - The composer keeps one `Attempt` for `{ groupId, text }` until the send succeeds or the
    text changes; a retry of the same text reuses its `command_id`.
- **Opening the page** while `needs_you` is true: the "needs you" mark clears on the next send
  only (Core's rule), so the page does not claim to have cleared it.

- [ ] Tests:
  - `lines_come_in_seq_order_with_their_speaker`
  - `a_notice_is_not_a_bubble`
  - `at_opens_the_member_menu_without_deleted_agents`
  - `the_log_does_not_jump_when_scrolled_up`
  - `a_thinking_member_appears_and_clears_without_a_new_message`: a member thread's
    operation starts, then ends; the mark shows, then goes, with no `MessageAdded`
  - `a_resend_after_a_lost_answer_reuses_the_command_id`
- [ ] Run; FAIL, implement, PASS.
- [ ] Commit `feat(web): the group page`.

### Task F-85: A member's permission request in the group

**Files:**
- Create: `web/src/app/conversation/permission-answer.tsx`,
  `web/src/app/conversation/permission-answer.test.tsx`
- Modify: `web/src/app/conversation/permission-card.tsx`,
  `web/src/app/conversation/permission-card.test.tsx`, `web/src/app/groups/group-page.tsx`,
  `web/src/app/conversation/contract.yaml`, `web/src/app/groups/contract.yaml`

**Interfaces:**
- `waitingPermissionsQuery` (`GET /api/aylul/permissions`, `web/src/api/permissions.ts`)
  already returns every AYLUL thread's waiting requests, hidden ones included; each carries
  `thread_id`. The page keeps those whose `thread_id` is one of its members' and shows each with
  the existing permission card under the log, with the member's name.
- **One answering part, two users.** `PermissionCard` reads a `ThreadEntry` (`entry.body`,
  `entry.card.request_id`); a group has a `WaitingPermission`, not an entry. Its answering
  part moves into `PermissionAnswer({ request }: { request: WaitingPermission })`: the
  options as buttons, the `Attempt`, `answerPermission(thread_id, request_id, option_id,
  command_id)`, and the `PERMISSION_NOT_FOUND` / `PERMISSION_SETTLED` handling it has today.
  `PermissionCard` keeps the entry's text and finds its waiting request as now, then renders
  `PermissionAnswer`. In the group, each request shows the member's name, then the request's
  `title` (what the agent wants to do; `WaitingPermission` carries it), then
  `PermissionAnswer`'s buttons. No
  `ThreadEntry` is made up.
- `permission-card.test.tsx` passes unchanged in what it checks.
- Places that list waiting requests as conversations (the conversation page's banner, if any)
  must not link to a hidden thread. The builder checks every use of `waitingPermissionsQuery`
  and sends a hidden thread's request to its group.

- [ ] Tests `a_members_request_is_answered_in_the_group` (the member's name and the request's
  `title` are shown before the buttons) and
  `permission_answer_answers_a_waiting_request` (in `permission-answer.test.tsx`).
- [ ] Run; FAIL, implement, PASS.
- [ ] Commit `feat(web): permission requests inside a group`.

---

## Desktop (AYLUL UI: `apps/desktop/`)

### Task U-9: Open a group from the desktop

**Files:**
- Modify: `apps/desktop/src-tauri/src/navigation.rs` (`MainDestination::Group { group_id }`),
  its contract and tests; the web side's handler of pending navigation

**Interfaces:**
- `MainDestination::Group { group_id }` navigates the main window to `/aylul/groups/<id>`.

- [ ] Test `a_group_destination_opens_the_group_page`.
- [ ] Commit `feat(desktop): open a group in the main window`.

### Task U-10: "Needs you" on the panel, and the alert

**Files:**
- Modify: `apps/desktop/src-tauri/src/presence/` (the watcher, its projection and tests), the
  native Core client in `presence/core_client/`,
  `apps/desktop/src/features/companion/panel/CompanionPanel.tsx`, and the contracts

**Interfaces:**
- **The watcher's feed:** it already subscribes to the AYLUL project's stream for permissions
  (`presence/watcher/feed.rs`, `Waiting`). `MessageAdded` is handled on that same subscription;
  no second one.
  - On `MessageAdded` it re-reads `GET /api/groups`, as it re-reads permissions on
    `PermissionsChanged`.
  - On connect and on reconnect it reads both lists after subscribing, as it does today.
- **What the pill counts:** one item per waiting thing.
  - A conversation's permission request is one item, as today.
  - A group is **one item** when it has `needs_you` or any waiting request on one of its
    member threads, however many of those it has.
- **Oldest first:** an item's time is when it started waiting.
  - A request's time is its `asked_at`.
  - A group's time is the earliest of its `needs_you_since` (B-46) and its member requests'
    `asked_at`.
  - Clicking the pill opens the oldest item: a conversation as today, or a group through U-9.
- **The tone plays only for something new, judged by identity, never by time.** The time
  above only orders the pill. A group's time can stay at an older request when a new call
  arrives, or move forward to a request already shown when an older one closes, so it cannot
  say what is new.
  - **A permission request** is new when its `(thread_id, request_id)` is not in the set the
    watcher has seen in this run of the app.
  - **A call to Mohammed** is new when the watcher reads a `MessageAdded` whose message addresses
    `Person`, with a `seq` above the highest it has seen for that group.
  - The watcher keeps those seen sets in memory for this run.
  - **A reconnect** rebuilds the count from the two lists. Requests already in the set ring
    nothing. A `MessageAdded` that the resumed stream delivers for the first time is new, and
    rings once for the batch, not once per message.
  - **The app's first connection is silent until `caught-up`.** The project feed starts at
    cursor 0 (`presence/watcher/feed.rs`), so it replays old `MessageAdded` events first, and
    `GET /api/groups` carries no message `seq`. Every request and every call that arrives before
    that subscription's first `caught-up` frame only fills the seen sets and the per-group
    highest `seq`. None of them rings.
  - **After that, in the same run,** the seen state and the feed's cursor (the last event handled)
    survive a reconnect, so only events after that cursor can be new.
- **A permission request on a hidden group thread** is counted once: as its group, not as a
  conversation. The pill reads `GET /api/aylul/permissions` today, and that list includes hidden
  threads, so without this the pill would open a hidden thread as a conversation. The watcher
  learns which threads are hidden from `GET /api/groups` (`members[].thread_id`), and the
  oldest-waiting target for such a request is `MainDestination::Group`.

- [ ] Tests:
  - `a_message_to_mohammed_raises_the_waiting_state`
  - `a_group_with_needs_you_and_two_requests_counts_once`
  - `a_reconnect_restores_the_count_without_an_old_tone`
  - `a_new_call_rings_while_an_older_request_waits`: a request at 10:00, then `@mohammed` at
    10:05; the tone plays for the call
  - `an_older_request_closing_rings_nothing`: requests at 10:00 and 10:03 are both seen; 10:00
    closes; no tone, though the group's time moves to 10:03
  - `startup_is_silent_and_a_call_during_a_drop_rings_once`:
    - the journal already holds a call to Mohammed when the app starts: no tone through the
      replay and `caught-up`, and the pill counts the group;
    - the feed then drops and a new call is written meanwhile: after the reconnect, exactly one
      tone.
  - `a_message_to_others_does_not`
  - `the_pill_counts_a_group_once`
  - `the_oldest_waiting_group_opens_the_group_page`
- [ ] Commit `feat(desktop): AYLUL tells Mohammed when a group needs him`.

---

## Acceptance

Mohammed runs `shadows serve` and the desktop. He then checks four things:

1. He creates a group of two agents, writes `@<slug> hello`, and sees the reply arrive without a
   refresh.
2. He writes a message that makes an agent mention `@mohammed`. The panel's pill counts it, and a
   click opens the group.
3. He stops one agent's provider to force a failure, and reads the `Notice`.
4. Slice 1 is done only after this run.

## Decided by Mohammed (2026-11-03)

- The groups list sits **above the agents** in the AYLUL sidebar, because a group is what asks
  for him.
- The alert is **the panel's pill and the existing tone**. A Windows toast is not in this plan;
  it comes later only if he finds he misses alerts.
