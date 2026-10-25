import type { HookEvent, HookReply } from '../protocol.ts';
import type { Route } from './server.ts';
import type { Sessions } from '../sessions/sessions.ts';
import type { Approvals } from '../approvals/approvals.ts';
import type { AgentSettings } from '../agents/settings.ts';

/** Records every event, then lets approvals answer the two that wait for the person. */
export function makeRoute(d: { sessions: Sessions; approvals: Approvals; settings: AgentSettings }): Route {
  return async (event: HookEvent, closed: AbortSignal): Promise<HookReply> => {
    const { step, at } = d.sessions.recordEvent(event);
    if (step) {
      d.approvals.observeStep(event.session_id, step.tool, step.target);
      d.settings.noteGrantUse(event, at);
    }
    if (event.hook_event_name === 'PermissionRequest') return d.approvals.request(event, closed);
    if (event.hook_event_name === 'PermissionDenied') return d.approvals.refusal(event, closed);
    return {};
  };
}
