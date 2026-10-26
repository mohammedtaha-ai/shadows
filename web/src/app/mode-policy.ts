// The project checklist and refused-permission line need mode names without an open session.
//
// This is Shadows' own decision, not a list of the harness's modes: a
// session's menus still come from what the harness offers, filtered by the
// daemon. The contract carries no copy of the policy, so this is the one
// place the client holds it.

export interface HarnessPolicy {
  /** The modes Shadows allows, in the order they are listed, each with the
   * line that says plainly what it lets the harness do. */
  readonly modes: readonly {
    readonly id: string
    readonly label: string
    readonly description: string
  }[]
  /** Every new conversation's mode; one that asks before a tool runs. */
  readonly initial: string | null
  /** The allowed mode that asks nothing: the one that would have let a refused
   * permission through. */
  readonly unattended: string | null
}

const CLAUDE_CODE_POLICY: HarnessPolicy = {
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
}

// AYLUL's brain is Claude Code on its own profile: the same two modes.
const POLICY: Readonly<Record<string, HarnessPolicy>> = {
  'claude-code': CLAUDE_CODE_POLICY,
  aylul: CLAUDE_CODE_POLICY,
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
