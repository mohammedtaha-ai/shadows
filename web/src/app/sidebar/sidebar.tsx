// One job: the left pane — the app's name, its projects, whether the daemon
// answers, and the way to the global settings.

import {useState} from 'react'
import { Link, useSearch } from '@tanstack/react-router'
import {isDesktop} from '../../desktop/runtime'
import {hideToTray} from '../../desktop/native'
import { Plus, Settings } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ConnectionIndicator } from '../daemon-status'
import { useOpenNewProject } from '../new-project/open-new-project'
import { AgentList } from './agent-list'
import { WorkspaceSwitcher } from './workspace-switcher'
import { useWorkspace } from './workspace'
import { ProjectList } from './project-list'
import { useOpenSettings } from '@/app/navigation'

export function Sidebar() {
  const [nativeError,setNativeError]=useState<string|null>(null)
  const openNewProject = useOpenNewProject()
  const { workspace, switchTo } = useWorkspace()
  const openSettings = useOpenSettings()
  const onSettings = useSearch({ from: '__root__' }).settings !== undefined
  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <header className="flex items-center justify-between px-4 pt-4 pb-3">
        <WorkspaceSwitcher workspace={workspace} onSwitch={switchTo} />
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
      {isDesktop()&&<div className="flex gap-3 px-4 py-2 text-xs">
        <Link to="/aylul/activity">Local activity</Link>
        <button onClick={()=>{void hideToTray().catch(()=>setNativeError('Unable to hide the window. Please retry.'))}}>Hide to tray</button>
      </div>}
      {nativeError&&<p role="alert" className="px-4 text-xs">{nativeError}</p>}
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
    </aside>
  )
}
