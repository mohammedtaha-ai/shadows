import type { ReactNode } from "react"
import { cn } from "cn"

// A block the design shows and Core does not serve yet. Its children are
// static placeholders (a dash, an empty bar, a disabled control): `inert`
// keeps them out of focus and clicks, and the dimming says they are not live.
function Planned({
  title,
  reason,
  className,
  children,
}: {
  title: string
  reason?: string
  className?: string
  children?: ReactNode
}) {
  return (
    <section
      aria-label={`${title} (planned)`}
      data-slot="planned"
      className={cn("space-y-2 rounded-xl border border-dashed border-border p-3", className)}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className="rounded-full border border-current px-2 py-0.5 text-xs font-semibold text-warning">
          Planned
        </span>
        <span className="text-xs text-muted-foreground">{title}</span>
      </div>
      {reason && <p className="text-xs text-muted-foreground">{reason}</p>}
      {children && (
        <div inert aria-hidden="true" className="pointer-events-none opacity-50 select-none">
          {children}
        </div>
      )}
    </section>
  )
}

export { Planned }
