import { randomBytes } from 'node:crypto';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export type Endpoint = { pipe: string; token: string; file: string };

/**
 * A fresh pipe name and token for this sidecar run, written to `dir/hook.json`.
 * `dir` is under %LOCALAPPDATA%, which only the person's Windows account can
 * read; the token, not the pipe's access list, keeps other accounts out.
 */
export function createEndpoint(dir: string): Endpoint {
  mkdirSync(dir, { recursive: true });
  const pipe = String.raw`\\.\pipe\aylul-` + randomBytes(16).toString('hex');
  const token = randomBytes(32).toString('hex');
  const file = join(dir, 'hook.json');
  writeFileSync(file, JSON.stringify({ pipe, token }));
  return { pipe, token, file };
}

export function removeEndpoint(endpoint: Endpoint): void {
  rmSync(endpoint.file, { force: true });
}
