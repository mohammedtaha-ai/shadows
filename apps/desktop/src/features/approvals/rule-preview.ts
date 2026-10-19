import type { Plan } from '../../platform/aylul';

export type RulePreview = Plan & { rule: string };

/** Keeps a displayed diff tied to the exact rule used to request it. */
export class RulePreviewGuard {
  #version = 0;

  invalidate(): void { this.#version++; }

  async load(rule: string, fetch: (rule: string) => Promise<Plan>): Promise<RulePreview | null> {
    const version = ++this.#version;
    const plan = await fetch(rule);
    return this.#version === version ? { ...plan, rule } : null;
  }

  approval(preview: RulePreview): { rule: string; before: string } {
    return { rule: preview.rule, before: preview.before };
  }

  shouldClear(error: unknown): boolean {
    if (String(error) !== 'settings_changed' && String(error) !== 'preview_mismatch') return false;
    this.invalidate();
    return true;
  }
}
