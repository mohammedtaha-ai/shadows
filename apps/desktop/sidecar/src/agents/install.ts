import { existsSync, readFileSync } from 'node:fs';
import { planEdit, type Plan } from './settings-file.ts';

/** Events AYLUL listens to, with the options each entry needs (spec §2, Global Constraints). */
export const HOOK_EVENTS: ReadonlyArray<readonly [string, Record<string, unknown>]> = [
  ['SessionStart', {}],
  ['SessionEnd', {}],
  ['Stop', {}],
  ['Notification', {}],
  ['PostToolUse', { async: true }],
  ['PermissionRequest', { timeout: 600 }],
  ['PermissionDenied', { timeout: 130 }],
];

type Group = { hooks?: unknown[]; [key: string]: unknown };
const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isAylul = (hook: unknown) =>
  isObject(hook) && typeof hook.command === 'string' && hook.command.toLowerCase().endsWith('aylul-hook.exe');

/** Replaces AYLUL's entries in ~/.claude/settings.json and keeps every other hook. */
export function planHookInstall(settingsPath: string, hookExe: string): Plan {
  return planEdit(settingsPath, settings => {
    const hooks = settings.hooks ?? {};
    if (!isObject(hooks)) throw new Error('settings_invalid');
    for (const [event, extra] of HOOK_EVENTS) {
      const groups = hooks[event] ?? [];
      if (!Array.isArray(groups)) throw new Error('settings_invalid');
      const kept: Group[] = [];
      for (const group of groups as Group[]) {
        if (!isObject(group) || !Array.isArray(group.hooks)) { kept.push(group); continue; }
        const others = group.hooks.filter(h => !isAylul(h));
        if (others.length > 0) kept.push({ ...group, hooks: others });
      }
      kept.push({ hooks: [{ type: 'command', command: hookExe, args: [], ...extra }] });
      hooks[event] = kept;
    }
    settings.hooks = hooks;
  });
}

/** True when every event has an entry running exactly this executable. */
export function hooksInstalled(settingsPath: string, hookExe: string): boolean {
  if (!existsSync(settingsPath)) return false;
  try {
    const hooks = JSON.parse(readFileSync(settingsPath, 'utf8')).hooks;
    return HOOK_EVENTS.every(([event]) => Array.isArray(hooks?.[event]) && hooks[event].some((g: Group) =>
      Array.isArray(g?.hooks) && g.hooks.some(h => isObject(h) && h.command === hookExe)));
  } catch {
    return false;
  }
}
