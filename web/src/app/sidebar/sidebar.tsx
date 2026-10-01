// One job: the left pane — the app's name, its projects, whether the daemon
// answers, and the way to the global settings.

import { Link, useLocation } from '@tanstack/react-router'
import { Plus, Settings } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ConnectionIndicator } from '../daemon-status'
import { useOpenNewProject } from '../new-project/open-new-project'
import { ProjectList } from './project-list'

export function Sidebar() {
  const openNewProject = useOpenNewProject()
  const onSettings = useLocation().pathname === '/settings'
  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <header className="flex items-center justify-between px-4 pt-4 pb-3">
        <span className="text-sm font-semibold tracking-wide">Shadows</span>
      </header>
      <div className="px-3 pb-3">
        <Button variant="secondary" size="sm" className="w-full justify-start" onClick={openNewProject}>
          <Plus />
          New project
        </Button>
      </div>
      <nav aria-label="Projects and conversations" className="flex-1 overflow-y-auto px-2 pb-3">
        <ProjectList />
      </nav>
      <footer className="flex items-center justify-between gap-3 border-t border-sidebar-border px-4 py-3">
        <div className="min-w-0">
          <ConnectionIndicator />
        </div>
        <Link
          to="/settings"
          aria-current={onSettings ? 'page' : undefined}
          className={`flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-xs transition-colors ${
            onSettings
              ? 'bg-secondary text-secondary-foreground'
              : 'text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
          }`}
        >
          <Settings aria-hidden className="size-3.5" />
          Settings
        </Link>
      </footer>
    </aside>
  )
}
