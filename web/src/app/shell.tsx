// One job: the two-pane frame every screen sits in.

import { Outlet } from '@tanstack/react-router'

export function Shell() {
  return (
    <div className="flex h-dvh overflow-hidden">
      <aside className="flex w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
        <header className="px-4 pt-4 pb-3">
          <span className="text-sm font-semibold tracking-wide">Shadows</span>
        </header>
        <nav aria-label="Projects and threads" className="flex-1 overflow-y-auto px-2" />
      </aside>
      <main className="flex min-w-0 flex-1 flex-col">
        <Outlet />
      </main>
    </div>
  )
}
