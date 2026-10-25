import { existsSync } from 'node:fs';
import type { Handlers } from './rpc.ts';
import type { Store } from './store/store.ts';
import type { Approvals } from './approvals/approvals.ts';
import { diffLines, expectBefore, fingerprint, writePlan, type Plan } from './agents/settings-file.ts';
import { hooksInstalled, planHookInstall } from './agents/install.ts';

export type HandlerDeps = {
  store: Store;
  approvals: Approvals;
  claudeSettings: string;
  hookExe: string | undefined;
  backupDir: string;
  now: () => Date;
};

const text = (params: Record<string, unknown>, key: string): string => {
  const value = params[key];
  if (typeof value !== 'string' || !value) throw new Error('bad_params');
  return value;
};

/** The calls AYLUL's window can make. Each reads its parameters and calls one owner. */
export function makeHandlers(d: HandlerDeps): Handlers {
  const exe = () => {
    if (!d.hookExe || !existsSync(d.hookExe)) throw new Error('hook_exe_missing');
    return d.hookExe;
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
    'refusal.preview': p => d.approvals.previewRule(text(p, 'id'), p.rule),
    'refusal.approve': p => {
      const id = text(p, 'id');
      const rule = text(p, 'rule').trim();
      const before = text(p, 'before');
      return d.approvals.approvePreview(id, rule, before);
    },
    'refusal.dismiss': p => { d.approvals.dismiss(text(p, 'id')); return null; },
    'hooks.preview': () => shown(planHookInstall(d.claudeSettings, exe())),
    'hooks.install': p => {
      const plan = expectBefore(planHookInstall(d.claudeSettings, exe()), text(p, 'before'));
      return { backupPath: writePlan(plan, d.backupDir, d.now()) };
    },
  };
}
