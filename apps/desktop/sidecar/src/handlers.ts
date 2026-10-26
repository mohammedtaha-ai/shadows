import type { Handlers } from './rpc.ts';
import type { Sessions } from './sessions/sessions.ts';
import type { Approvals } from './approvals/approvals.ts';
import type { AgentSettings } from './agents/settings.ts';

export type HandlerDeps = { sessions: Sessions; approvals: Approvals; settings: AgentSettings };

const text = (params: Record<string, unknown>, key: string): string => {
  const value = params[key];
  if (typeof value !== 'string' || !value) throw new Error('bad_params');
  return value;
};

/** The calls AYLUL's window can make. Each reads its parameters and calls one owner. */
export function makeHandlers(d: HandlerDeps): Handlers {
  return {
    state: () => ({
      sessions: d.sessions.list(),
      pending: d.approvals.pending(),
      hooksInstalled: d.settings.installed(),
    }),
    steps: p => d.sessions.steps(text(p, 'sessionId')),
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
    'hooks.preview': () => d.settings.previewHooks(),
    'hooks.install': p => d.settings.installHooks(() => text(p, 'before')),
  };
}
