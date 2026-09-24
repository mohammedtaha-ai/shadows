// One job: Shadows' mode policy per harness (spec §12.4), for the places the
// client names modes without an open session to ask: the project page's
// checklist of allowed modes, and a refused permission's line.
//
// This is Shadows' own decision, not a list of the harness's modes: a
// session's menus still come from what the harness offers, filtered by the
// daemon. The contract carries no copy of the policy, so this is the one
// place the client holds it.

export interface HarnessPolicy {
  /** The modes Shadows allows, in the order they are listed, each with the
   * line that says plainly what it lets the harness do (spec §12.5). */
  readonly modes: readonly {
    readonly id: string
    readonly label: string
    readonly description: string
  }[]
  /** Every new conversation's mode; one that asks before a tool runs. */
  readonly initial: string | null
  /** The allowed mode that does not ask (spec §12.2): the one that would
   * have let a refused permission through. */
  readonly unattended: string | null
}

const POLICY: Readonly<Record<string, HarnessPolicy>> = {
  'claude-code': {
    modes: [
      // Claude Code's own acceptEdits: file commands inside the project
      // folder, `rm` included, run without a permission request, so Shadows
      // never sees them; anything else is asked, and Shadows refuses it.
      {
        id: 'acceptEdits',
        label: 'Accept edits',
        description:
          'Claude Code edits, creates and deletes files in the project folder without asking. Other commands are refused.',
      },
      { id: 'auto', label: 'Auto', description: 'Claude Code decides on its own; nothing is asked.' },
    ],
    initial: 'acceptEdits',
    unattended: 'auto',
  },
}

/** A harness's policy; one with no decided modes (Codex, today) allows none. */
export function policyOf(harness: string): HarnessPolicy {
  return POLICY[harness] ?? { modes: [], initial: null, unattended: null }
}

/** How the policy names `mode`; the id itself for a mode it does not list. */
export function modeLabel(policy: HarnessPolicy, mode: string): string {
  return policy.modes.find((m) => m.id === mode)?.label ?? mode
}

/** What the policy says `mode` allows; `null` for a mode it does not list. */
export function modeDescription(policy: HarnessPolicy, mode: string): string | null {
  return policy.modes.find((m) => m.id === mode)?.description ?? null
}
