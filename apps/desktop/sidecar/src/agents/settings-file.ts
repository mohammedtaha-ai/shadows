import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { basename, dirname, join, win32 } from 'node:path';
import { isNarrowRule } from './rules.ts';

export type Plan = { path: string; before: string; after: string };
type Settings = Record<string, unknown>;

export const settingsFor = (project: string) => win32.join(project, '.claude', 'settings.local.json');

const isObject = (v: unknown): v is Settings => typeof v === 'object' && v !== null && !Array.isArray(v);

/** Reads the settings file and returns the change without writing it. */
export function planEdit(path: string, change: (settings: Settings) => void): Plan {
  const before = existsSync(path) ? readFileSync(path, 'utf8') : '';
  let settings: unknown = {};
  if (before.trim()) {
    try { settings = JSON.parse(before); } catch { throw new Error('settings_invalid'); }
  }
  if (!isObject(settings)) throw new Error('settings_invalid');
  const copy = structuredClone(settings);
  change(copy);
  return { path, before, after: JSON.stringify(copy, null, 2) + '\n' };
}

/** Adds `rule` to permissions.allow, keeping everything else. */
export function planAllowRule(path: string, rule: string): Plan {
  if (!isNarrowRule(rule)) throw new Error('rule_not_narrow');
  return planEdit(path, settings => {
    const permissions = settings.permissions ?? {};
    if (!isObject(permissions)) throw new Error('settings_invalid');
    const allow = permissions.allow ?? [];
    if (!Array.isArray(allow)) throw new Error('settings_invalid');
    if (!allow.includes(rule)) allow.push(rule);
    permissions.allow = allow;
    settings.permissions = permissions;
  });
}

/** The sha-256 of the file text a preview showed; the approval sends it back. */
export function fingerprint(text: string): string {
  return createHash('sha256').update(text).digest('hex');
}

/** The plan, if the file still holds what the person previewed; settings_changed otherwise. */
export function expectBefore(plan: Plan, seen: string): Plan {
  if (fingerprint(plan.before) !== seen) throw new Error('settings_changed');
  return plan;
}

/** Backs up, then replaces the file in one rename. Refuses if it changed since the plan. */
export function writePlan(plan: Plan, backupDir: string, now: Date): string {
  const current = existsSync(plan.path) ? readFileSync(plan.path, 'utf8') : '';
  if (current !== plan.before) throw new Error('settings_changed');
  let backupPath = '';
  if (existsSync(plan.path)) {
    mkdirSync(backupDir, { recursive: true });
    const tag = createHash('sha256').update(plan.path.toLowerCase()).digest('hex').slice(0, 8);
    backupPath = join(backupDir, `${now.toISOString().replace(/[:.]/g, '-')}-${tag}-${basename(plan.path)}`);
    copyFileSync(plan.path, backupPath);
  }
  mkdirSync(dirname(plan.path), { recursive: true });
  const temp = `${plan.path}.aylul-${process.pid}.tmp`;
  writeFileSync(temp, plan.after);
  renameRetrying(temp, plan.path);
  return backupPath;
}

/**
 * Windows refuses a rename with EPERM, EACCES or EBUSY while another program
 * (Claude Code, an antivirus) has the file open; that passes within moments.
 */
function renameRetrying(from: string, to: string): void {
  for (let attempt = 1; ; attempt++) {
    try { renameSync(from, to); return; } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (attempt >= 10 || !['EPERM', 'EACCES', 'EBUSY'].includes(code ?? '')) {
        rmSync(from, { force: true });
        throw error;
      }
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
    }
  }
}

/** Changed lines only, from a longest-common-subsequence line diff. */
export function diffLines(plan: Plan): string[] {
  const a = plan.before.trim() ? JSON.stringify(JSON.parse(plan.before), null, 2).split('\n') : [];
  const b = plan.after.trimEnd().split('\n');
  const lcs: number[][] = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--)
    for (let j = b.length - 1; j >= 0; j--)
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
  const out: string[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) { i++; j++; }
    else if (lcs[i + 1][j] >= lcs[i][j + 1]) out.push('- ' + a[i++]);
    else out.push('+ ' + b[j++]);
  }
  while (i < a.length) out.push('- ' + a[i++]);
  while (j < b.length) out.push('+ ' + b[j++]);
  return out;
}
