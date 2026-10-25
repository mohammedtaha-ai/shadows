// AYLUL's laptop process. Tauri starts it and talks to it over stdio; aylul-hook reaches it over the pipe.
import { homedir } from 'node:os';
import { join } from 'node:path';
import { createActivityApp } from './app.ts';
import { createEndpoint, removeEndpoint } from './hooks/endpoint.ts';
import { startHookServer } from './hooks/server.ts';
import { makeRoute } from './hooks/route.ts';
import { makeHandlers } from './handlers.ts';
import { serveRpc } from './rpc.ts';

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
