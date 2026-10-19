import type { HookEvent } from '../protocol.ts';

export type Step = { at: string; tool: string; target: string; summary: string };

const TARGET_KEYS = ['file_path', 'notebook_path', 'path', 'url', 'pattern'];

/** Tool name, its target and one line. Never file contents, patches or tool output. */
export function trimStep(event: HookEvent, at: string): Step {
  const tool = event.tool_name ?? 'unknown';
  const input = event.tool_input ?? {};
  if (tool === 'Bash' || tool === 'PowerShell') {
    const command = oneLine(String(input.command ?? ''));
    const description = typeof input.description === 'string' ? oneLine(input.description) : '';
    return { at, tool, target: command, summary: description || command };
  }
  const key = TARGET_KEYS.find(k => typeof input[k] === 'string');
  const target = key ? targetOf(key, String(input[key])) : '';
  return { at, tool, target, summary: target ? `${tool} ${target}` : tool };
}

/** A URL keeps scheme, host and path only: its query and fragment can carry tokens. */
function targetOf(key: string, value: string): string {
  if (key !== 'url') return oneLine(value);
  try {
    const url = new URL(value);
    url.search = '';
    url.hash = '';
    url.username = '';
    url.password = '';
    return oneLine(url.toString());
  } catch {
    return oneLine(value);
  }
}

export function oneLine(text: string, max = 160): string {
  const line = text.replace(/\s+/g, ' ').trim();
  return line.length > max ? line.slice(0, max - 1) + '…' : line;
}
