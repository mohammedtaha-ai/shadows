import { existsSync } from 'node:fs';
import { win32 } from 'node:path';
import type { Handlers } from './rpc.ts';
import type { Store } from './store/store.ts';
import type { Approvals } from './approvals/approvals.ts';
import { isNarrowRule } from './agents/rules.ts';
import { diffLines, expectBefore, fingerprint, planAllowRule, writePlan, type Plan } from './agents/settings-file.ts';
import { hooksInstalled, planHookInstall } from './agents/install.ts';

export type HandlerDeps = {
  store: Store;
  approvals: Approvals;
  claudeSettings: string;
  hookExe: string | undefined;
  backupDir: string;
  now: () => Date;
};

export const settingsFor = (project: string) => win32.join(project, '.claude', 'settings.local.json');

const text = (params: Record<string, unknown>, key: string): string => {
  const value = params[key];
  if (typeof value !== 'string' || !value) throw new Error('bad_params');
  return value;
};

/** The calls AYLUL's window can make. Each reads its parameters and calls one owner. */
export function makeHandlers(d: HandlerDeps): Handlers {
  const rulePreviews = new Map<string, { rule: string; before: string }>();
  const exe = () => {
    if (!d.hookExe || !existsSync(d.hookExe)) throw new Error('hook_exe_missing');
    return d.hookExe;
  };
  const refusal = (id: string) => {
    const item = d.approvals.pending().find(p => p.id === id && p.kind === 'refusal');
    if (!item) throw new Error('not_pending');
    if (!item.project) throw new Error('project_unknown');
    return item;
  };
  // What a preview shows, plus the fingerprint its approval must send back.
  const shown = (plan: Plan) => ({ path: plan.path, lines: diffLines(plan), before: fingerprint(plan.before) });
  return {
    state: () => ({
      sessions: d.store.listSessions(),
      pending: d.approvals.pending(),
      hooksInstalled: !!d.hookExe && hooksInstalled(d.claudeSettings, d.hookExe),
    }),
    steps: p => d.store.steps(text(p, 'sessionId')),
    'request.answer': p => {
      const behavior = p.behavior;
      if (behavior !== 'allow' && behavior !== 'deny') throw new Error('bad_params');
      d.approvals.answer(text(p, 'id'), behavior);
      return null;
    },
    'refusal.preview': p => {
      const item = refusal(text(p, 'id'));
      const rule = text(p, 'rule').trim();
      if (!isNarrowRule(rule)) throw new Error('rule_too_broad');
      const preview = shown(planAllowRule(settingsFor(item.project), rule));
      rulePreviews.set(item.id, { rule, before: preview.before });
      return preview;
    },
    'refusal.approve': p => {
      const id = text(p, 'id');
      const rule = text(p, 'rule').trim();
      const before = text(p, 'before');
      const preview = rulePreviews.get(id);
      if (!preview || preview.rule !== rule || preview.before !== before) throw new Error('preview_mismatch');
      const result = d.approvals.approveRule(id, rule, before);
      rulePreviews.delete(id);
      return result;
    },
    'refusal.dismiss': p => { const id = text(p, 'id'); d.approvals.dismiss(id); rulePreviews.delete(id); return null; },
    'hooks.preview': () => shown(planHookInstall(d.claudeSettings, exe())),
    'hooks.install': p => {
      const plan = expectBefore(planHookInstall(d.claudeSettings, exe()), text(p, 'before'));
      return { backupPath: writePlan(plan, d.backupDir, d.now()) };
    },
  };
}
