import type { HookEvent } from '../protocol.ts';
import { applyEvent } from './session.ts';
import type { SessionsStore } from './store.ts';

export class Sessions {
  #store: SessionsStore;
  #now: () => Date;
  constructor(store: SessionsStore, now: () => Date) { this.#store = store; this.#now = now; }
  list() { return this.#store.listSessions(); }
  steps(id: string) { return this.#store.steps(id); }
  recordEvent(event: HookEvent) {
    const at = this.#now().toISOString();
    const { session, step } = applyEvent(this.#store.getSession(event.session_id), event, at);
    this.#store.saveSession(session);
    if (step) this.#store.addStep(session.id, step);
    return { step, at };
  }
}
