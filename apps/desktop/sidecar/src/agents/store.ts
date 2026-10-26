import type { DatabaseSync } from 'node:sqlite';
import { randomUUID } from 'node:crypto';
export type RuleRow = { id: string; project: string; rule: string; backupPath: string; at: string };

/** Private persistence for the agents owner. */
export class SettingsStore {
  #db: DatabaseSync;
  constructor(db: DatabaseSync) { this.#db = db; }

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

}

function plain(rows: unknown[]): Array<Record<string, unknown>> {
  return rows.map(row => ({ ...(row as Record<string, unknown>) }));
}
