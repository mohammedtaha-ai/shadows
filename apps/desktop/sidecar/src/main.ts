// AYLUL's laptop process. Tauri starts it and talks to it over stdio; aylul-hook reaches it over the pipe.
import { homedir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { openDatabase } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { createEndpoint, removeEndpoint } from './hooks/endpoint.ts';
import { startHookServer } from './hooks/server.ts';
import { makeRoute } from './hooks/route.ts';
import { SessionsStore } from './sessions/store.ts';
import { ApprovalsStore } from './approvals/store.ts';
import { SettingsStore } from './agents/store.ts';
import { Sessions } from './sessions/sessions.ts';
import { AgentSettings, type SettingsDeps } from './agents/settings.ts';
import { pathToFileURL } from 'node:url';
import { makeHandlers } from './handlers.ts';
import { serveRpc } from './rpc.ts';

export function createActivityApp(file: string, deps: SettingsDeps & { newId?: () => string; requestHoldMs?: number; refusalHoldMs?: number }) {
  const db = openDatabase(file);
  const sessions = new Sessions(new SessionsStore(db), deps.now);
  const decisions = new ApprovalsStore(db);
  const settings = new AgentSettings(new SettingsStore(db), deps);
  const approvals = new Approvals({
    now: deps.now, newId: deps.newId ?? randomUUID, store: decisions,
    writeRule: (project, rule, seen) => settings.writeRule(project, rule, seen),
    requestHoldMs: deps.requestHoldMs ?? 580_000,
    refusalHoldMs: deps.refusalHoldMs ?? 120_000,
  });
  return { sessions, approvals, settings, close: () => db.close() };
}

async function start() {
  // AYLUL_DATA_DIR keeps a smoke run away from the real hook.json and database.
  const dataDir = process.env.AYLUL_DATA_DIR ?? join(process.env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local'), 'AYLUL');
  const claudeDir = process.env.CLAUDE_CONFIG_DIR ?? join(homedir(), '.claude');
  const app = createActivityApp(join(dataDir, 'aylul.db'), {
    now: () => new Date(), backupDir: join(dataDir, 'backups'),
    claudeSettings: join(claudeDir, 'settings.json'), hookExe: process.env.AYLUL_HOOK_EXE,
  });
  const endpoint = createEndpoint(dataDir);
  const server = await startHookServer(endpoint.pipe, endpoint.token, makeRoute(app));
  serveRpc(process.stdin, process.stdout, makeHandlers(app), () => {
    server.close();
    removeEndpoint(endpoint);
    app.close();
    process.exit(0);
  });
  process.stderr.write('aylul-sidecar: ready\n');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await start();
