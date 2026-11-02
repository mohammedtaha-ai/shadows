// One job: the left pane — the app's name, its projects, whether the daemon
// answers, and the way to the global settings.

import { motion, useReducedMotion } from 'motion/react'
import { useRef, useState } from 'react'
import { SidebarToggle } from './sidebar-toggle'
import { useSearch } from '@tanstack/react-router'
import { DesktopSidebarActions } from '@/app/desktop'
import { Plus, Search, Settings, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
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
  // Display only: the lists narrow what the client already holds.
  const [filter, setFilter] = useState('')
  const [searchOpen, setSearchOpen] = useState(false)
  const searchInputRef = useRef<HTMLInputElement | null>(null)
  const closeSearch = () => { setSearchOpen(false); setFilter('') }
  const toggleSearch = () => {
    if (searchOpen) closeSearch()
    else { setSearchOpen(true); requestAnimationFrame(() => searchInputRef.current?.focus()) }
  }
  return (
    <>
    {narrow && <motion.button type="button" aria-label="Close sidebar" className="fixed inset-0 z-30 bg-background/70 backdrop-blur-xs" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }} onClick={() => { onHide(); requestAnimationFrame(() => document.querySelector<HTMLButtonElement>('main [aria-label="Show sidebar"]')?.focus()) }} />}
    <motion.aside id="app-sidebar" className={`flex shrink-0 flex-col overflow-hidden relative border-r border-sidebar-border/80 bg-sidebar text-sidebar-foreground shadow-xl ${narrow ? 'fixed inset-y-0 left-0 z-40 w-72 max-w-[85vw]' : 'w-72'}`} initial={narrow ? { x: '-100%' } : { width: 0 }} animate={narrow ? { x: 0 } : { width: 288 }} exit={narrow ? { x: '-100%' } : { width: 0 }} transition={{ duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }}>
      <div aria-hidden className="pointer-events-none absolute -top-20 -left-16 size-48 rounded-full bg-accent-line/12 blur-3xl" />
      <header className="relative flex items-center justify-between border-b border-sidebar-border/50 px-4 pt-4 pb-3">
        <WorkspaceSwitcher workspace={workspace} onSwitch={switchTo} />
        <div className="flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger render={
              <Button variant="ghost" size="icon" aria-label="Search sidebar" aria-expanded={searchOpen} onClick={toggleSearch}
                className={searchOpen || filter !== '' ? 'bg-accent/40 text-accent-line' : undefined}>
                <Search aria-hidden />
              </Button>
            } />
            <TooltipContent>{searchOpen ? 'Close search' : 'Search'}</TooltipContent>
          </Tooltip>
          <SidebarToggle shown onToggle={onToggle} />
        </div>
      </header>
      <div className="space-y-2 px-3 pt-3 pb-2.5">
        {searchOpen && (
          <div className="relative flex items-center">
            <Search aria-hidden className="pointer-events-none absolute left-2.5 size-3.5 text-accent-line/70" />
            <input
              ref={searchInputRef}
              type="search"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); closeSearch() } }}
              placeholder={workspace === 'aylul' ? 'Filter agents…' : 'Filter projects…'}
              aria-label="Filter sidebar"
              className="h-7 w-full rounded-lg border border-accent-line/35 bg-input/60 pr-7 pl-8 text-xs text-sidebar-foreground placeholder:text-faint-foreground focus:border-accent-line/70 focus:outline-none"
            />
            <button
              type="button"
              aria-label="Clear sidebar filter"
              onClick={() => { if (filter !== '') { setFilter(''); searchInputRef.current?.focus() } else setSearchOpen(false) }}
              className="absolute right-2 rounded p-0.5 text-faint-foreground hover:text-sidebar-foreground"
            >
              <X aria-hidden className="size-3" />
            </button>
          </div>
        )}
        <Button
          variant="secondary"
          size="sm"
          className="w-full justify-start gap-2 rounded-xl border border-accent-line/45 bg-gradient-to-r from-accent/40 to-accent-softer text-sidebar-foreground transition-all hover:border-accent-line/75"
          onClick={() => workspace === 'aylul' ? openSettings('agents', 'new') : openNewProject()}
        >
          <Plus className="size-4 text-accent-line" />
          {workspace === 'aylul' ? 'New agent' : 'New project'}
        </Button>
      </div>
      <nav aria-label={workspace === 'aylul' ? 'Agents and conversations' : 'Projects and conversations'} className="flex-1 overflow-y-auto px-2.5 pb-3">
        {workspace === 'aylul' ? <AgentList filter={filter} /> : <ProjectList filter={filter} />}
      </nav>
      <DesktopSidebarActions />
      <footer className="flex items-center justify-between gap-3 border-t border-sidebar-border/70 bg-sidebar/90 px-4 py-3">
        <div className="min-w-0">
          <ConnectionIndicator />
        </div>
        <button
          type="button"
          data-settings-trigger
          onClick={() => openSettings('general')}
          aria-current={onSettings ? 'page' : undefined}
          className={`flex shrink-0 items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-medium transition-all ${
            onSettings
              ? 'pill-active'
              : 'border-border/60 bg-sidebar-accent/40 text-muted-foreground hover:border-accent-line/40 hover:bg-sidebar-accent hover:text-sidebar-foreground'
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
