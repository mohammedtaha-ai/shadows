// One job: the sidebar title as the workspace menu — each workspace with its
// label, what it is for, and a check on the current one.

import { ChevronsUpDown } from 'lucide-react'
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { Workspace } from './workspace'

const WORKSPACES: { id: Workspace; label: string; description: string }[] = [
  { id: 'shadows', label: 'Shadows', description: 'Plan and build projects' },
  { id: 'aylul', label: 'AYLUL', description: 'Your agents and their conversations' },
]

export function WorkspaceSwitcher({ workspace, onSwitch }: {
  workspace: Workspace
  onSwitch: (next: Workspace) => void
}) {
  const current = WORKSPACES.find((w) => w.id === workspace) ?? WORKSPACES[0]!
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label="Workspace"
        className="group -mx-1.5 flex items-center gap-1.5 rounded-md px-1.5 py-1 outline-none transition-colors hover:bg-sidebar-accent focus-visible:ring-3 focus-visible:ring-ring/50 data-[popup-open]:bg-sidebar-accent"
      >
        {/* The app's wordmark, now naming the workspace shown below it. */}
        <span className="bg-gradient-to-r from-foreground via-foreground to-accent-line bg-clip-text text-sm font-semibold tracking-wide text-transparent">
          {current.label}
        </span>
        <ChevronsUpDown
          aria-hidden
          className="size-3.5 text-faint-foreground transition-colors group-hover:text-sidebar-foreground"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="min-w-64">
        <DropdownMenuRadioGroup
          value={workspace}
          onValueChange={(v) => { if (v !== workspace) onSwitch(v as Workspace) }}
        >
          {WORKSPACES.map((w) => (
            <DropdownMenuRadioItem key={w.id} value={w.id}>
              <span className="flex flex-col gap-0.5">
                <span className="text-sm">{w.label}</span>
                <span className="text-xs text-muted-foreground">{w.description}</span>
              </span>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
