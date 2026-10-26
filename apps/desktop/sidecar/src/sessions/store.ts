import type { DatabaseSync } from 'node:sqlite';
import type { Session } from './session.ts';
import type { Step } from './steps.ts';
export const KEPT_STEPS = 50;

/** Private persistence for the sessions owner. */
export class SessionsStore {
  #db: DatabaseSync;
  constructor(db: DatabaseSync) { this.#db = db; }

  saveSession(session: Session): void {
    this.#db.prepare(`INSERT INTO sessions (id, body, updated_at) VALUES (?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at`)
      .run(session.id, JSON.stringify(session), session.updatedAt);
  }

  getSession(id: string): Session | undefined {
    const row = this.#db.prepare('SELECT body FROM sessions WHERE id = ?').get(id) as { body: string } | undefined;
    return row ? JSON.parse(row.body) as Session : undefined;
  }

  listSessions(limit = 50): Session[] {
    const rows = this.#db.prepare('SELECT body FROM sessions ORDER BY updated_at DESC, id LIMIT ?').all(limit) as Array<{ body: string }>;
    return rows.map(r => JSON.parse(r.body) as Session);
  }

  addStep(sessionId: string, step: Step): void {
    this.#db.prepare('INSERT INTO steps (session_id, body) VALUES (?, ?)').run(sessionId, JSON.stringify(step));
    this.#db.prepare(`DELETE FROM steps WHERE session_id = ? AND seq NOT IN
      (SELECT seq FROM steps WHERE session_id = ? ORDER BY seq DESC LIMIT ?)`).run(sessionId, sessionId, KEPT_STEPS);
  }

  steps(sessionId: string): Step[] {
    const rows = this.#db.prepare('SELECT body FROM steps WHERE session_id = ? ORDER BY seq').all(sessionId) as Array<{ body: string }>;
    return rows.map(r => JSON.parse(r.body) as Step);
  }

}
