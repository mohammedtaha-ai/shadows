import { randomUUID } from 'node:crypto';
import { openDatabase } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { SessionsStore } from './sessions/store.ts';
import { ApprovalsStore } from './approvals/store.ts';
import { SettingsStore } from './agents/store.ts';
import { Sessions } from './sessions/sessions.ts';
import { AgentSettings, type SettingsDeps } from './agents/settings.ts';

export type ActivityDeps = SettingsDeps & { newId?: () => string; requestHoldMs?: number; refusalHoldMs?: number };

/** The sidecar's one composition: every service and its private store, built on one connection. */
export function createActivityApp(file: string, deps: ActivityDeps) {
  const db = openDatabase(file);
  const sessions = new Sessions(new SessionsStore(db), deps.now);
  const settings = new AgentSettings(new SettingsStore(db), deps);
  const approvals = new Approvals({
    now: deps.now, newId: deps.newId ?? randomUUID, store: new ApprovalsStore(db),
    writeRule: (project, rule, seen) => settings.writeRule(project, rule, seen),
    requestHoldMs: deps.requestHoldMs ?? 580_000,
    refusalHoldMs: deps.refusalHoldMs ?? 120_000,
  });
  return { sessions, approvals, settings, close: () => db.close() };
}
