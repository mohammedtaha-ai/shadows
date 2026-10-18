import { createInterface } from 'node:readline';

export type Handlers = Record<string, (params: Record<string, unknown>) => unknown>;

export const ERROR_CODES = new Set(['unknown_method', 'bad_params', 'bad_request', 'not_pending', 'rule_too_broad',
  'project_unknown', 'settings_invalid', 'settings_changed', 'preview_mismatch', 'hook_exe_missing']);

/** One JSON line per call on input, one per answer on output. Errors leave as stable codes only. */
export function serveRpc(input: NodeJS.ReadableStream, output: NodeJS.WritableStream, handlers: Handlers, onEnd: () => void): void {
  const write = (value: object) => output.write(JSON.stringify(value) + '\n');
  const lines = createInterface({ input, crlfDelay: Infinity });
  lines.on('line', line => {
    let message: { id?: unknown; method?: unknown; params?: unknown };
    try { message = JSON.parse(line); } catch { write({ id: null, error: 'bad_request' }); return; }
    const id = message.id ?? null;
    const method = typeof message.method === 'string' && Object.hasOwn(handlers, message.method) ? handlers[message.method] : undefined;
    if (!method) { write({ id, error: 'unknown_method' }); return; }
    const params = typeof message.params === 'object' && message.params !== null ? message.params as Record<string, unknown> : {};
    Promise.resolve()
      .then(() => method(params))
      .then(result => write({ id, result: result ?? null }), error => write({ id, error: code(error) }));
  });
  lines.on('close', onEnd);
}

function code(error: unknown): string {
  const message = error instanceof Error ? error.message : '';
  if (ERROR_CODES.has(message)) return message;
  process.stderr.write('aylul-sidecar: action_failed\n');
  return 'action_failed';
}
