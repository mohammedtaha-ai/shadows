import { createServer, type Server, type Socket } from 'node:net';
import { timingSafeEqual } from 'node:crypto';
import type { HookEvent, HookReply } from '../protocol.ts';

/** Answers one hook event. `closed` aborts if the hook hangs up before the answer. */
export type Route = (event: HookEvent, closed: AbortSignal) => Promise<HookReply>;

const MAX_LINE = 1024 * 1024;

export function startHookServer(pipe: string, token: string, route: Route): Promise<Server> {
  const expected = Buffer.from(token);
  const server = createServer(socket => serve(socket, expected, route));
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(pipe, () => {
      server.off('error', reject);
      resolve(server);
    });
  });
}

function serve(socket: Socket, expected: Buffer, route: Route): void {
  const closed = new AbortController();
  let buffer = '';
  let handled = false;
  let answered = false;
  socket.setEncoding('utf8');
  socket.on('error', () => {});
  socket.on('close', () => { if (!answered) closed.abort(); });
  socket.on('data', chunk => {
    if (handled) return;
    buffer += chunk;
    if (buffer.length > MAX_LINE) { socket.destroy(); return; }
    const end = buffer.indexOf('\n');
    if (end < 0) return;
    handled = true;
    const event = parse(buffer.slice(0, end), expected);
    if (!event) { socket.destroy(); return; }
    route(event, closed.signal)
      .catch((): HookReply => ({}))
      .then(reply => {
        if (socket.destroyed) return;
        answered = true;
        socket.end(JSON.stringify(reply) + '\n');
      });
  });
}

function parse(line: string, expected: Buffer): HookEvent | undefined {
  try {
    const value = JSON.parse(line);
    const got = Buffer.from(typeof value?.token === 'string' ? value.token : '');
    if (value?.v !== 1 || got.length !== expected.length || !timingSafeEqual(got, expected)) return undefined;
    const event = value.event;
    if (typeof event?.hook_event_name !== 'string' || typeof event?.session_id !== 'string') return undefined;
    const projectDir = typeof value.project_dir === 'string' && value.project_dir ? value.project_dir : undefined;
    return { ...event, project_dir: projectDir } as HookEvent;
  } catch {
    return undefined;
  }
}
