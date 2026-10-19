import type { HookEvent } from '../protocol.ts';
import { trimStep, type Step } from './steps.ts';

export type SessionState = 'running' | 'waiting' | 'ended';
export type Session = {
  id: string;
  agent: 'claude_code';
  project: string;
  state: SessionState;
  startedAt: string;
  updatedAt: string;
  lastStep: Step | null;
  lastMessage: string | null;
};

export const MAX_MESSAGE = 2000;
const WAITING_NOTICES = new Set(['permission_prompt', 'idle_prompt', 'elicitation_dialog', 'agent_needs_input']);

/** The session after one hook event, and the step it added, if any. */
export function applyEvent(current: Session | undefined, event: HookEvent, at: string): { session: Session; step?: Step } {
  const session: Session = current
    ? { ...current, updatedAt: at }
    : { id: event.session_id, agent: 'claude_code', project: '', state: 'running',
        startedAt: at, updatedAt: at, lastStep: null, lastMessage: null };
  // The project root wins; cwd (which follows `cd`) only fills in when the root never arrived.
  if (event.project_dir) session.project = event.project_dir;
  else if (!session.project && event.cwd) session.project = event.cwd;
  switch (event.hook_event_name) {
    case 'SessionStart':
      session.state = 'running';
      break;
    case 'PostToolUse': {
      const step = trimStep(event, at);
      session.lastStep = step;
      session.state = 'running';
      return { session, step };
    }
    case 'PermissionRequest':
      session.state = 'waiting';
      break;
    case 'Notification':
      if (WAITING_NOTICES.has(event.notification_type ?? '')) session.state = 'waiting';
      break;
    case 'Stop':
      session.state = 'waiting';
      if (typeof event.last_assistant_message === 'string') session.lastMessage = clip(event.last_assistant_message);
      break;
    case 'SessionEnd':
      session.state = 'ended';
      break;
  }
  return { session };
}

function clip(text: string): string {
  return text.length > MAX_MESSAGE ? text.slice(0, MAX_MESSAGE - 1) + '…' : text;
}
