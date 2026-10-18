// AYLUL's laptop process. Tauri starts it and talks to it over stdio; aylul-hook reaches it over the pipe.
import { homedir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { Store } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { createEndpoint, removeEndpoint } from './hooks/endpoint.ts';
import { startHookServer } from './hooks/server.ts';
import { makeRoute } from './hooks/route.ts';
import { expectBefore, planAllowRule, writePlan } from './agents/settings-file.ts';
import { makeHandlers, settingsFor } from './handlers.ts';
import { serveRpc } from './rpc.ts';

// AYLUL_DATA_DIR keeps a smoke run away from the real hook.json and database.
const dataDir = process.env.AYLUL_DATA_DIR ?? join(process.env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local'), 'AYLUL');
const backupDir = join(dataDir, 'backups');
const claudeDir = process.env.CLAUDE_CONFIG_DIR ?? join(homedir(), '.claude');
const now = () => new Date();

const store = new Store(join(dataDir, 'aylul.db'));
const approvals = new Approvals({
  now,
  newId: randomUUID,
  record: (item, outcome) => store.recordDecision(item, outcome, now().toISOString()),
  writeRule: (project, rule, seen) => {
    const backupPath = writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), backupDir, now());
    return { ruleId: store.addRule(project, rule, backupPath, now().toISOString()), backupPath };
  },
  requestHoldMs: 580_000,
  refusalHoldMs: 120_000,
});
const endpoint = createEndpoint(dataDir);
const server = await startHookServer(endpoint.pipe, endpoint.token, makeRoute({ store, approvals, now }));

serveRpc(process.stdin, process.stdout, makeHandlers({
  store, approvals, claudeSettings: join(claudeDir, 'settings.json'), hookExe: process.env.AYLUL_HOOK_EXE, backupDir, now,
}), () => {
  server.close();
  removeEndpoint(endpoint);
  store.close();
  process.exit(0);
});
process.stderr.write('aylul-sidecar: ready\n');
