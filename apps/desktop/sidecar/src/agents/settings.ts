import { existsSync } from 'node:fs';
import type { HookEvent } from '../protocol.ts';
import type { SettingsStore } from './store.ts';
import { proposeRule } from './rules.ts';
import { hooksInstalled, planHookInstall } from './install.ts';
import { diffLines, fingerprint, expectBefore, planAllowRule, settingsFor, writePlan } from './settings-file.ts';

export type SettingsDeps = { claudeSettings: string; hookExe: string | undefined; backupDir: string; now: () => Date };

export class AgentSettings {
  #store: SettingsStore;
  #d: SettingsDeps;
  constructor(store: SettingsStore, deps: SettingsDeps) { this.#store = store; this.#d = deps; }
  installed() { return !!this.#d.hookExe && hooksInstalled(this.#d.claudeSettings, this.#d.hookExe); }
  #hookPlan() {
    if (!this.#d.hookExe || !existsSync(this.#d.hookExe)) throw new Error('hook_exe_missing');
    return planHookInstall(this.#d.claudeSettings, this.#d.hookExe);
  }
  previewHooks() {
    const plan = this.#hookPlan();
    return { path: plan.path, lines: diffLines(plan), before: fingerprint(plan.before) };
  }
  installHooks(before: () => string) {
    // Resolve the executable before checking the fingerprint parameter, as the RPC requires.
    const plan = expectBefore(this.#hookPlan(), before());
    return { backupPath: writePlan(plan, this.#d.backupDir, this.#d.now()) };
  }
  writeRule(project: string, rule: string, seen: string) {
    const backupPath = writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), this.#d.backupDir, this.#d.now());
    return { ruleId: this.#store.addRule(project, rule, backupPath, this.#d.now().toISOString()), backupPath };
  }
  noteGrantUse(event: HookEvent, at: string) {
    const project = event.project_dir ?? '';
    const covering = proposeRule(event.tool_name ?? '', event.tool_input ?? {}, project);
    if (!covering) return;
    const rule = this.#store.rulesFor(project).find(r => r.rule === covering);
    if (rule) this.#store.addGrantUse(rule.id, event.session_id, event.tool_use_id ?? '', at);
  }
}
