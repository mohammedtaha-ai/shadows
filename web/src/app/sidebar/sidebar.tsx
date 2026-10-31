// One job: the left pane — the app's name, its projects, whether the daemon
// answers, and the way to the global settings.

import { motion, useReducedMotion } from 'motion/react'
import { SidebarToggle } from './sidebar-toggle'
import { useSearch } from '@tanstack/react-router'
import { DesktopSidebarActions } from '@/app/desktop'
import { Plus, Settings } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ConnectionIndicator } from '../daemon-status'
import { useOpenNewProject } from '@/app/new-project'
import { AgentList } from './agent-list'
import { WorkspaceSwitcher } from './workspace-switcher'
import { useWorkspace } from './workspace'
import { ProjectList } from './project-list'
import { useOpenSettings } from '@/app/navigation'

export function Sidebar({ narrow, onToggle, onHide }: { narrow: boolean; onToggle: () => void; onHide: () => void }) {
  const reducedMotion = useReducedMotion()
  const openNewProject = useOpenNewProject()
  const { workspace, switchTo } = useWorkspace()
  const openSettings = useOpenSettings()
  const onSettings = useSearch({ from: '__root__' }).settings !== undefined
  return (
    <>
    {narrow && <motion.button type="button" aria-label="Close sidebar" className="fixed inset-0 z-30 bg-black/40" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }} onClick={() => { onHide(); requestAnimationFrame(() => document.querySelector<HTMLButtonElement>('main [aria-label="Show sidebar"]')?.focus()) }} />}
    <motion.aside id="app-sidebar" className={`flex shrink-0 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar text-sidebar-foreground ${narrow ? 'fixed inset-y-0 left-0 z-40 w-72 max-w-[85vw]' : 'w-72'}`} initial={narrow ? { x: '-100%' } : { width: 0 }} animate={narrow ? { x: 0 } : { width: 288 }} exit={narrow ? { x: '-100%' } : { width: 0 }} transition={{ duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }}>
      <header className="flex items-center justify-between px-4 pt-4 pb-3">
        <WorkspaceSwitcher workspace={workspace} onSwitch={switchTo} />
        <SidebarToggle shown onToggle={onToggle} />
      </header>
      <div className="px-3 pb-3">
        <Button
          variant="secondary"
          size="sm"
          className="w-full justify-start gap-2 shadow-xs transition-all hover:bg-secondary/90"
          onClick={() => workspace === 'aylul' ? openSettings('agents', 'new') : openNewProject()}
        >
          <Plus className="size-4" />
          {workspace === 'aylul' ? 'New agent' : 'New project'}
        </Button>
      </div>
      <nav aria-label={workspace === 'aylul' ? 'Agents and conversations' : 'Projects and conversations'} className="flex-1 overflow-y-auto px-2 pb-3">
        {workspace === 'aylul' ? <AgentList /> : <ProjectList />}
      </nav>
      <DesktopSidebarActions />
      <footer className="flex items-center justify-between gap-3 border-t border-sidebar-border px-4 py-3">
        <div className="min-w-0">
          <ConnectionIndicator />
        </div>
        <button
          type="button"
          data-settings-trigger
          onClick={() => openSettings('general')}
          aria-current={onSettings ? 'page' : undefined}
          className={`flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-xs transition-colors ${
            onSettings
              ? 'bg-secondary text-secondary-foreground'
              : 'text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
          }`}
        >
          <Settings aria-hidden className="size-3.5" />
          Settings
        </button>
      </footer>
    </motion.aside>
    </>
  )
}
