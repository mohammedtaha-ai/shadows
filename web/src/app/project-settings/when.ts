// One job: a stored time as the person reads it on the settings page.

/** `iso` in the reader's locale; the raw value if it does not parse. */
export function when(iso: string): string {
  const at = new Date(iso)
  return Number.isNaN(at.getTime())
    ? iso
    : at.toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
}

const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['second', 60],
  ['minute', 60],
  ['hour', 24],
  ['day', Number.POSITIVE_INFINITY],
]

/** How long ago `iso` was, as "2 min. ago" in the reader's locale; the raw
 * value if it does not parse. */
export function ago(iso: string, now: number = Date.now()): string {
  const at = new Date(iso).getTime()
  if (Number.isNaN(at)) return iso
  const format = new Intl.RelativeTimeFormat([], { numeric: 'auto', style: 'short' })
  let amount = Math.round((at - now) / 1000)
  for (const [unit, size] of STEPS) {
    if (Math.abs(amount) < size) return format.format(amount, unit)
    amount = Math.round(amount / size)
  }
  return iso
}
