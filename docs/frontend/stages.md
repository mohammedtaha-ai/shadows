# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

No task is open. Still open outside any task: the light palette and the real
desktop login-item behaviour need their own app and OS acceptance.

Finished work: F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

Branch `feature/web-polish`: fixes and small improvements the user finds while using the app.

### F-50 · The message box keeps the caret after Send
- Status: todo
- Plan: none; a bug found in use, 2026-10-30
- Read: `web/src/app/conversation/contract.yaml` (Composer, useSend)
- Touch: `web/src/app/conversation/composer.tsx`, its test file, `web/src/app/conversation/contract.yaml`
- Needs: none
- Found: `composer.tsx` sets `disabled={inFlight}` on the textarea. A disabled field loses focus
  to `BODY` and gets no focus back when it is enabled again, so after every Send or queue the
  person must click the box to type. Measured in the browser: right after Enter,
  `document.activeElement` is `BODY` and the box is disabled; after the send, the box is
  enabled and focus is still `BODY`.
- Done when: a test sends with Enter while the box has focus and finds the box still focused
  (and not editable while the send is in flight) once the send settles, for both a send and a
  queued message; `npx vitest run src/app/conversation` passes.

### F-51 · A streamed reply shows as it arrives, without freezing the page
- Status: todo
- Plan: none; a bug found in use, 2026-10-30
- Read: `web/src/stream/contract.yaml` (delta), `web/src/app/conversation/contract.yaml`
  (Messages, Entry, PlanCard)
- Touch: files under `web/src/app/conversation/` and `web/src/stream/` that the cause lies in,
  their tests and their `contract.yaml`; report before touching anything else
- Needs: none
- Found: Core is not the cause. Recorded on `/api/subscribe` for one Planner reply: 100 `delta`
  frames over 8.8 s, the largest gap 0.14 s. In the browser on the same thread (a long
  conversation holding a plan card), the visible text went 14 → 54 characters, then one long
  task blocked the main thread for 4549 ms, then the text jumped to 786. Other long tasks:
  317 ms at send, 126 ms, 175 ms at the end. Find what one render does for 4.5 s while
  deltas arrive (profile it; do not guess), and fix that cause.
- Done when: a test drives many deltas into a thread that holds a plan card and long markdown
  and proves the cause is gone (for example the expensive part renders once, not per delta);
  `npx vitest run src/app/conversation src/stream` passes; and in the browser a reply on that
  thread grows steadily with no long task over 200 ms.

