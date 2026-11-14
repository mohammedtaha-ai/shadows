import type { ComponentProps } from 'react'
import { cn } from 'cn'

/** Decorative geometry; the enclosing loading region supplies its accessible name. */
export function Skeleton({ className, ...props }: ComponentProps<'span'>) {
  return <span {...props} aria-hidden="true" data-slot="skeleton"
    className={cn('block rounded-md bg-muted-foreground/15 motion-safe:animate-pulse', className)} />
}

type Layout = 'rows' | 'cards' | 'form' | 'detail' | 'conversation' | 'graph'

/** Inline placeholders stay valid inside labels, links and other phrasing content. */
export function SkeletonText({ label, className }: { label: string; className?: string }) {
  return <span role="status" aria-label={label} aria-busy="true" className="inline-flex align-middle">
    <span className="sr-only">{label}</span>
    <Skeleton className={cn('h-3 w-24', className)} />
  </span>
}

/** A single announced loading region with non-interactive placeholders. */
export function SkeletonContent({ label = 'Loading…', layout = 'rows', className }: {
  label?: string
  layout?: Layout
  className?: string
}) {
  return <div role="status" aria-label={label} aria-busy="true"
    className={cn('w-full min-w-0', className)}>
    <span className="sr-only">{label}</span>
    <div aria-hidden="true">
      {layout === 'rows' && <div className="space-y-3 py-2">
        {['w-4/5', 'w-3/5', 'w-2/3'].map(width => <div key={width} className="flex items-center gap-3">
          <Skeleton className="size-6 shrink-0 rounded-lg" />
          <Skeleton className={cn('h-3', width)} />
        </div>)}
      </div>}
      {layout === 'cards' && <div className="mx-auto max-w-5xl space-y-6">
        <div className="flex items-center gap-4 rounded-2xl border border-border p-6">
          <Skeleton className="size-12 shrink-0 rounded-2xl" />
          <div className="flex-1 space-y-3"><Skeleton className="h-3 w-28" /><Skeleton className="h-6 w-2/3 max-w-80" /><Skeleton className="h-3 w-1/2" /></div>
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          {[0, 1, 2, 3].map(index => <div key={index} className="space-y-5 rounded-2xl border border-border p-5">
            <div className="space-y-3"><Skeleton className="h-4 w-1/2" /><Skeleton className="h-3 w-4/5" /></div>
            <div className="flex gap-3 border-t border-border pt-4"><Skeleton className="h-8 w-28 rounded-lg" /><Skeleton className="h-8 w-24 rounded-lg" /></div>
          </div>)}
        </div>
      </div>}
      {layout === 'form' && <div className="space-y-5 py-2">
        {[0, 1, 2].map(index => <div key={index} className="space-y-2.5">
          <Skeleton className="h-3 w-28" /><Skeleton className="h-10 w-full rounded-lg" />
        </div>)}
      </div>}
      {layout === 'detail' && <div className="space-y-6">
        <div className="space-y-3"><Skeleton className="h-3 w-32" /><Skeleton className="h-7 w-2/3 max-w-lg" /></div>
        <div className="flex gap-3 border-b border-border pb-3"><Skeleton className="h-8 w-20" /><Skeleton className="h-8 w-20" /><Skeleton className="h-8 w-20" /></div>
        <div className="space-y-3 rounded-xl border border-border p-5"><Skeleton className="h-4 w-1/3" /><Skeleton className="h-3 w-full" /><Skeleton className="h-3 w-5/6" /><Skeleton className="h-3 w-2/3" /><Skeleton className="mt-6 h-24 w-full rounded-lg" /></div>
      </div>}
      {layout === 'conversation' && <div className="flex min-h-80 flex-col gap-7">
        <div className="flex items-center gap-3 border-b border-border pb-4"><Skeleton className="size-9 rounded-xl" /><Skeleton className="h-5 w-48" /></div>
        <div className="ml-auto w-3/5 space-y-3 rounded-2xl border border-border p-4"><Skeleton className="h-3 w-full" /><Skeleton className="h-3 w-2/3" /></div>
        <div className="w-4/5 space-y-3"><Skeleton className="h-3 w-full" /><Skeleton className="h-3 w-5/6" /><Skeleton className="h-3 w-3/5" /></div>
        <Skeleton className="mt-auto h-24 w-full rounded-2xl" />
      </div>}
      {layout === 'graph' && <div className="space-y-6">
        <Skeleton className="h-6 w-48" />
        <div className="flex min-h-72 flex-wrap items-center justify-center gap-6 rounded-2xl border border-border p-6">
          {[0, 1, 2].map(index => <div key={index} className="w-48 space-y-4 rounded-xl border border-border p-4">
            <Skeleton className="h-4 w-2/3" /><Skeleton className="h-3 w-full" /><Skeleton className="h-3 w-4/5" /><Skeleton className="h-6 w-20" />
          </div>)}
        </div>
      </div>}
    </div>
  </div>
}
