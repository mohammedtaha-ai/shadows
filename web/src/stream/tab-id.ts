// Send a page-load id with every turn so live `plan-show` frames target the sending tab.
//
// Transport state: never stored, not even in `sessionStorage`, so a
// reload is a new tab and a duplicated tab never shares its original's id.

let id: string | null = null

/** This page load's random id. */
export function tabId(): string {
  id ??= crypto.randomUUID()
  return id
}
