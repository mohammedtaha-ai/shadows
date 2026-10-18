import { DatabaseSync } from 'node:sqlite';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';
import type { Session } from '../sessions/session.ts';
import type { Step } from '../sessions/steps.ts';
import type { Outcome, Pending } from '../approvals/approvals.ts';

export const KEPT_STEPS = 50;
export type RuleRow = { id: string; project: string; rule: string; backupPath: string; at: string };
export type Decision = { itemId: string; sessionId: string; kind: string; tool: string; target: string; outcome: string; at: string };

const SCHEMA = `
CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, body TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS steps (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS decisions (seq INTEGER PRIMARY KEY AUTOINCREMENT, item_id TEXT NOT NULL, session_id TEXT NOT NULL,
  kind TEXT NOT NULL, tool TEXT NOT NULL, target TEXT NOT NULL, outcome TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rules (id TEXT PRIMARY KEY, project TEXT NOT NULL, rule TEXT NOT NULL, backup_path TEXT NOT NULL,
  approved_by TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS grant_uses (seq INTEGER PRIMARY KEY AUTOINCREMENT, rule_id TEXT NOT NULL, session_id TEXT NOT NULL,
  tool_use_id TEXT NOT NULL, at TEXT NOT NULL);
`;

/** AYLUL's laptop records. Every list is ordered by an explicit key. */
export class Store {
  #db: DatabaseSync;

  constructor(file: string) {
    if (file !== ':memory:') mkdirSync(dirname(file), { recursive: true });
    this.#db = new DatabaseSync(file);
    this.#db.exec(SCHEMA);
  }

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

  addRule(project: string, rule: string, backupPath: string, at: string): string {
    const id = randomUUID();
    this.#db.prepare(`INSERT INTO rules (id, project, rule, backup_path, approved_by, at) VALUES (?, ?, ?, ?, 'person', ?)`)
      .run(id, project, rule, backupPath, at);
    return id;
  }

  rulesFor(project: string): RuleRow[] {
    return plain(this.#db.prepare(`SELECT id, project, rule, backup_path AS backupPath, at FROM rules
      WHERE project = ? ORDER BY at, id`).all(project)) as RuleRow[];
  }

  addGrantUse(ruleId: string, sessionId: string, toolUseId: string, at: string): void {
    this.#db.prepare('INSERT INTO grant_uses (rule_id, session_id, tool_use_id, at) VALUES (?, ?, ?, ?)')
      .run(ruleId, sessionId, toolUseId, at);
  }

  grantUses(): Array<{ ruleId: string; sessionId: string; toolUseId: string }> {
    return plain(this.#db.prepare(`SELECT rule_id AS ruleId, session_id AS sessionId, tool_use_id AS toolUseId
      FROM grant_uses ORDER BY seq`).all()) as Array<{ ruleId: string; sessionId: string; toolUseId: string }>;
  }

  close(): void { this.#db.close(); }
}

function plain(rows: unknown[]): Array<Record<string, unknown>> {
  return rows.map(row => ({ ...(row as Record<string, unknown>) }));
}
