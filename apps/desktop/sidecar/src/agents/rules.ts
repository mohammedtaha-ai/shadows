import { win32 } from 'node:path';

const SHELL_TOOLS = new Set(['Bash', 'PowerShell']);
const FILE_TOOLS = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit', 'Read']);
/** Characters Claude Code's path rules read as a pattern, not as themselves. */
const PATTERN = /[*?[\]]/;

/**
 * The narrowest Claude Code permission rule covering this call, or null when
 * none can be proposed safely. `project` is the session's project root
 * (CLAUDE_PROJECT_DIR); a `/path` rule in settings.local.json is anchored there.
 */
export function proposeRule(tool: string, input: Record<string, unknown>, project: string): string | null {
  if (SHELL_TOOLS.has(tool)) {
    const command = typeof input.command === 'string' ? input.command.trim() : '';
    if (!command || /[\r\n*]/.test(command) || command.includes(')')) return null;
    return `${tool}(${command})`;
  }
  if (FILE_TOOLS.has(tool)) {
    const file = input.file_path ?? input.notebook_path;
    if (typeof file !== 'string' || !project) return null;
    const rel = win32.relative(project, file);
    if (!rel || rel.startsWith('..') || win32.isAbsolute(rel)) return null;
    const path = rel.split('\\').join('/');
    if (PATTERN.test(path)) return null;
    return `${tool === 'Read' ? 'Read' : 'Edit'}(/${path})`;
  }
  if (tool === 'WebFetch') {
    try { return `WebFetch(domain:${new URL(String(input.url)).hostname})`; } catch { return null; }
  }
  if (/^mcp__[^*\s()]+__[^*\s()]+$/.test(tool)) return tool;
  return null;
}

/**
 * True only for the shapes proposeRule makes: an exact command, one file
 * anchored at the project root, one host, or one MCP tool. Everything else,
 * including any wildcard, is refused; this is the only check before a rule
 * the person typed is written.
 */
export function isNarrowRule(rule: string): boolean {
  const text = rule.trim();
  if (/^mcp__[^*\s()]+__[^*\s()]+$/.test(text)) return true;
  const match = /^(\w+)\((.*)\)$/s.exec(text);
  if (!match) return false;
  const [, tool, arg] = match;
  if (!arg || arg.trim() !== arg || arg.includes(')') || /[\r\n*]/.test(arg)) return false;
  if (SHELL_TOOLS.has(tool)) return true;
  if (tool === 'Edit' || tool === 'Read')
    return /^\/[^/~]/.test(arg) && !arg.endsWith('/') && !PATTERN.test(arg) && !arg.split('/').includes('..');
  if (tool === 'WebFetch') return /^domain:[a-z0-9-]+(\.[a-z0-9-]+)+$/i.test(arg);
  return false;
}
