import type { DatabaseSync } from 'node:sqlite';
import type { Pending, Outcome } from './approvals.ts';
export type Decision = { itemId: string; sessionId: string; kind: string; tool: string; target: string; outcome: string; at: string };

/** Private persistence for the approvals owner. */
export class ApprovalsStore {
  #db: DatabaseSync;
  constructor(db: DatabaseSync) { this.#db = db; }

  recordDecision(item: Pending, outcome: Outcome, at: string): void {
    this.#db.prepare(`INSERT INTO decisions (item_id, session_id, kind, tool, target, outcome, at)
      VALUES (?, ?, ?, ?, ?, ?, ?)`).run(item.id, item.sessionId, item.kind, item.tool, item.target, outcome, at);
  }

  // node:sqlite returns rows with a null prototype; `plain` copies them into ordinary objects.
  decisions(): Decision[] {
    return plain(this.#db.prepare(`SELECT item_id AS itemId, session_id AS sessionId, kind, tool, target, outcome, at
      FROM decisions ORDER BY seq`).all()) as Decision[];
  }

  /** A step after a terminal answer shows the person allowed it there. */
  markTerminalAllowed(sessionId: string, tool: string, target: string): boolean {
    const result = this.#db.prepare(`UPDATE decisions SET outcome = 'answered_in_terminal_allowed' WHERE seq =
      (SELECT seq FROM decisions WHERE outcome = 'answered_in_terminal' AND session_id = ? AND tool = ? AND target = ?
       ORDER BY seq DESC LIMIT 1)`).run(sessionId, tool, target);
    return Number(result.changes) > 0;
  }

}

function plain(rows: unknown[]): Array<Record<string, unknown>> {
  return rows.map(row => ({ ...(row as Record<string, unknown>) }));
}
