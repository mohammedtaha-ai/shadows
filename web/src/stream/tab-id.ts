// One job: this tab's id (§13.9) — made once per page load, sent with every
// turn, so a live `plan-show` frame can say which tab it is for.
//
// Transport state (§2.10): never stored, not even in `sessionStorage`, so a
// reload is a new tab and a duplicated tab never shares its original's id.

let id: string | null = null

/** This page load's random id. */
export function tabId(): string {
  id ??= crypto.randomUUID()
  return id
}
