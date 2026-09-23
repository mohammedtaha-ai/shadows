// One job: the left pane — the app's name, its projects, and whether the
// daemon answers.

import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ConnectionIndicator } from '../daemon-status'
import { useOpenNewProject } from '../new-project/open-new-project'
import { ProjectList } from './project-list'

export function Sidebar() {
  const openNewProject = useOpenNewProject()
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
      <footer className="border-t border-sidebar-border px-4 py-3">
        <ConnectionIndicator />
      </footer>
    </aside>
  )
}
