import { invoke } from '@tauri-apps/api/core';

export type Step = { at: string; tool: string; target: string; summary: string };
export type Session = {
  id: string; agent: 'claude_code'; project: string; state: 'running' | 'waiting' | 'ended';
  startedAt: string; updatedAt: string; lastStep: Step | null; lastMessage: string | null;
};
export type Pending = {
  id: string; kind: 'request' | 'refusal'; sessionId: string; project: string; tool: string; target: string;
  reason: string | null; proposedRule: string | null; at: string; held: boolean;
};
export type AylulState = { sessions: Session[]; pending: Pending[]; hooksInstalled: boolean };
/** A preview; `before` goes back with the approval so a file changed since is not overwritten. */
export type Plan = { path: string; lines: string[]; before: string };

const call = <T>(method: string, params: object = {}) => invoke<T>('aylul_call', { method, params });

export const getState = () => call<AylulState>('state');
export const answerRequest = (id: string, behavior: 'allow' | 'deny') => call<null>('request.answer', { id, behavior });
export const previewRule = (id: string, rule: string) => call<Plan>('refusal.preview', { id, rule });
export const approveRule = (id: string, rule: string, before: string) =>
  call<{ backupPath: string }>('refusal.approve', { id, rule, before });
export const dismissRefusal = (id: string) => call<null>('refusal.dismiss', { id });
export const previewHooks = () => call<Plan>('hooks.preview');
export const installHooks = (before: string) => call<{ backupPath: string }>('hooks.install', { before });

const MESSAGES: Record<string, string> = {
  sidecar_unavailable: "AYLUL's local service is not running. Restart AYLUL.",
  sidecar_timeout: "AYLUL's local service did not answer. Try again.",
  not_pending: 'This item was already answered or has ended.',
  rule_too_broad: 'That rule would allow too much. Name the exact command, one file as /path from the project root, or one site.',
  project_unknown: 'AYLUL does not know this session\'s project folder, so it cannot write the rule.',
  settings_invalid: 'The settings file is not valid JSON. AYLUL did not change it.',
  settings_changed: 'The settings file changed since the preview. Preview again.',
  preview_mismatch: 'The rule changed since the preview. Preview it again.',
  hook_exe_missing: 'aylul-hook.exe was not found. Build it first.',
};
export const aylulMessage = (code: string) => MESSAGES[code] ?? 'Unable to complete this action. Please retry.';
