import type { ApprovalsStore } from './store.ts';
import type { HookEvent, HookReply } from '../protocol.ts';
import { oneLine, trimStep } from '../sessions/steps.ts';
import { isNarrowRule, proposeRule } from '../agents/rules.ts';
import { diffLines, fingerprint, planAllowRule, settingsFor } from '../agents/settings-file.ts';

export type Pending = {
  id: string;
  kind: 'request' | 'refusal';
  sessionId: string;
  project: string;
  tool: string;
  target: string;
  reason: string | null;
  proposedRule: string | null;
  at: string;
  held: boolean;
};
export type Outcome = 'allowed' | 'denied' | 'answered_in_terminal' | 'timed_out' | 'listed' | 'rule_written' | 'dismissed';
export type ApprovalsDeps = {
  now: () => Date;
  newId: () => string;
  /** Writes the rule if the file still matches the fingerprint `seen` the person previewed. */
  writeRule: (project: string, rule: string, seen: string) => { ruleId: string; backupPath: string };
  requestHoldMs: number;
  refusalHoldMs: number;
} & ({ store: ApprovalsStore; record?: never } | {
  store?: never;
  record: (item: Pending, outcome: Outcome) => void;
});

type Item = Pending & { reply?: (reply: HookReply) => void };

const ALLOW: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: { behavior: 'allow' } } } };
const DENY: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest',
  decision: { behavior: 'deny', message: 'The person denied this in AYLUL.' } } } };
const RETRY: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } };

/** Requests and refusals waiting for the person. Only the person's answer allows anything. */
export class Approvals {
  #items = new Map<string, Item>();
  #rulePreviews = new Map<string, { rule: string; before: string }>();
  #d: ApprovalsDeps;

  constructor(deps: ApprovalsDeps) { this.#d = deps; }

  #record(item: Pending, outcome: Outcome): void {
    if (this.#d.store) this.#d.store.recordDecision(item, outcome, this.#d.now().toISOString());
    else this.#d.record(item, outcome);
  }

  observeStep(sessionId: string, tool: string, target: string): void {
    this.#d.store?.markTerminalAllowed(sessionId, tool, target);
  }

  pending(): Pending[] {
    return [...this.#items.values()]
      .map(({ reply: _reply, ...item }) => item)
      .sort((a, b) => a.at.localeCompare(b.at) || a.id.localeCompare(b.id));
  }

  request(event: HookEvent, closed: AbortSignal): Promise<HookReply> {
    const item = this.#item('request', event, null);
    return this.#hold(item, closed, this.#d.requestHoldMs,
      () => { this.#items.delete(item.id); this.#record(item, 'timed_out'); },
      () => { this.#items.delete(item.id); this.#record(item, 'answered_in_terminal'); });
  }

  refusal(event: HookEvent, closed: AbortSignal): Promise<HookReply> {
    // No root, no rule: it could not be written to the settings file Claude Code reads.
    const rule = event.project_dir ? proposeRule(event.tool_name ?? '', event.tool_input ?? {}, event.project_dir) : null;
    const item = this.#item('refusal', event, rule);
    const list = () => this.#record(item, 'listed');
    return this.#hold(item, closed, this.#d.refusalHoldMs, list, list);
  }

  answer(id: string, behavior: 'allow' | 'deny'): void {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'request' || !item.reply) throw new Error('not_pending');
    const reply = item.reply;
    this.#items.delete(id);
    this.#record(item, behavior === 'allow' ? 'allowed' : 'denied');
    reply(behavior === 'allow' ? ALLOW : DENY);
  }

  previewRule(id: string, value: unknown): { path: string; lines: string[]; before: string } {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'refusal') throw new Error('not_pending');
    if (!item.project) throw new Error('project_unknown');
    if (typeof value !== 'string' || !value) throw new Error('bad_params');
    const rule = value.trim();
    if (!isNarrowRule(rule)) throw new Error('rule_too_broad');
    const plan = planAllowRule(settingsFor(item.project), rule);
    const preview = { path: plan.path, lines: diffLines(plan), before: fingerprint(plan.before) };
    this.#rulePreviews.set(item.id, { rule, before: preview.before });
    return preview;
  }

  approvePreview(id: string, rule: string, before: string): { backupPath: string } {
    const preview = this.#rulePreviews.get(id);
    if (!preview || preview.rule !== rule || preview.before !== before) throw new Error('preview_mismatch');
    const result = this.#approveRule(id, rule, before);
    this.#rulePreviews.delete(id);
    return result;
  }

  // Private so that every write passes the preview admission above.
  #approveRule(id: string, rule: string, seen: string): { backupPath: string } {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'refusal') throw new Error('not_pending');
    if (!isNarrowRule(rule)) throw new Error('rule_too_broad');
    // Without the root a rule could land in a subfolder's settings, which Claude Code never reads.
    if (!item.project) throw new Error('project_unknown');
    const written = this.#d.writeRule(item.project, rule.trim(), seen);
    const reply = item.reply;
    this.#items.delete(id);
    this.#record(item, 'rule_written');
    reply?.(RETRY);
    return { backupPath: written.backupPath };
  }

  dismiss(id: string): void {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'refusal') throw new Error('not_pending');
    const reply = item.reply;
    this.#items.delete(id);
    this.#record(item, 'dismissed');
    reply?.({});
    this.#rulePreviews.delete(id);
  }

  #item(kind: Pending['kind'], event: HookEvent, proposedRule: string | null): Item {
    const step = trimStep(event, '');
    const shellCommand = (event.tool_name === 'Bash' || event.tool_name === 'PowerShell') &&
      typeof event.tool_input?.command === 'string' ? oneLine(event.tool_input.command, Number.MAX_SAFE_INTEGER) : null;
    return { id: this.#d.newId(), kind, sessionId: event.session_id, project: event.project_dir ?? '',
      tool: step.tool, target: kind === 'request' && shellCommand !== null ? shellCommand : step.target,
      reason: event.reason ?? null, proposedRule,
      at: this.#d.now().toISOString(), held: true };
  }

  /** Waits for the person, the hook hanging up, or the hold time; whichever is first. */
  #hold(item: Item, closed: AbortSignal, ms: number, onTimeout: () => void, onClosed: () => void): Promise<HookReply> {
    this.#items.set(item.id, item);
    return new Promise(resolve => {
      let done = false;
      const finish = (reply: HookReply) => {
        if (done) return;
        done = true;
        clearTimeout(timer);
        closed.removeEventListener('abort', hangUp);
        item.reply = undefined;
        item.held = false;
        resolve(reply);
      };
      const timer = setTimeout(() => { onTimeout(); finish({}); }, ms);
      const hangUp = () => { onClosed(); finish({}); };
      item.reply = finish;
      if (closed.aborted) hangUp();
      else closed.addEventListener('abort', hangUp, { once: true });
    });
  }
}
