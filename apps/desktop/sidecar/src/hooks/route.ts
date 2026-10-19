import type { HookEvent, HookReply } from '../protocol.ts';
import type { Route } from './server.ts';
import type { Store } from '../store/store.ts';
import type { Approvals } from '../approvals/approvals.ts';
import { applyEvent } from '../sessions/session.ts';
import { proposeRule } from '../agents/rules.ts';

/** Records every event, then lets approvals answer the two that wait for the person. */
export function makeRoute(d: { store: Store; approvals: Approvals; now: () => Date }): Route {
  return async (event: HookEvent, closed: AbortSignal): Promise<HookReply> => {
    const at = d.now().toISOString();
    const { session, step } = applyEvent(d.store.getSession(event.session_id), event, at);
    d.store.saveSession(session);
    if (step) {
      d.store.addStep(session.id, step);
      d.store.markTerminalAllowed(session.id, step.tool, step.target);
      noteGrantUse(d.store, event, at);
    }
    if (event.hook_event_name === 'PermissionRequest') return d.approvals.request(event, closed);
    if (event.hook_event_name === 'PermissionDenied') return d.approvals.refusal(event, closed);
    return {};
  };
}

function noteGrantUse(store: Store, event: HookEvent, at: string): void {
  const project = event.project_dir ?? '';
  const covering = proposeRule(event.tool_name ?? '', event.tool_input ?? {}, project);
  if (!covering) return;
  const rule = store.rulesFor(project).find(r => r.rule === covering);
  if (rule) store.addGrantUse(rule.id, event.session_id, event.tool_use_id ?? '', at);
}
