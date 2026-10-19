// One job: a labelled control in the AYLUL settings forms — the label above,
// the control, and the hint under it.

import type { ComponentProps, ReactNode } from 'react'

export function Field({ label, hint, children }: {
  label: ReactNode
  hint?: ReactNode
  children: ReactNode
}) {
  return (
    <label className="grid gap-1.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
      {hint !== undefined && <span className="text-xs text-faint-foreground">{hint}</span>}
    </label>
  )
}

/** A native select drawn like the app's `Input`. */
export function Select({ className = '', ...props }: ComponentProps<'select'>) {
  return (
    <select
      {...props}
      className={`h-8 w-full min-w-0 rounded-lg border border-input bg-input-background px-2 text-sm text-foreground outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 ${className}`}
    />
  )
}

/** A multi-line field drawn like the app's `Input`. */
export function TextArea({ className = '', ...props }: ComponentProps<'textarea'>) {
  return (
    <textarea
      {...props}
      className={`min-h-24 w-full min-w-0 rounded-lg border border-input bg-input-background px-2.5 py-1.5 font-mono text-sm text-foreground outline-none transition-colors placeholder:text-faint-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 ${className}`}
    />
  )
}
