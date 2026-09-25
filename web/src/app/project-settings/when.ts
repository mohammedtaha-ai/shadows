// One job: a stored time as the person reads it on the settings page.

/** `iso` in the reader's locale; the raw value if it does not parse. */
export function when(iso: string): string {
  const at = new Date(iso)
  return Number.isNaN(at.getTime())
    ? iso
    : at.toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
}
