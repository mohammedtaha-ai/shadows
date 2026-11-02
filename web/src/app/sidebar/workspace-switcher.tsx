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

/** The leaf mark beside the workspace name; its colours follow the theme tokens. */
export function LeafMark({ className = 'size-5' }: { className?: string }) {
  return (
    <svg viewBox="0 0 28 28" fill="none" aria-hidden className={className}>
      <defs>
        <linearGradient id="leaf-grad" x1="3" y1="2" x2="24" y2="26" gradientUnits="userSpaceOnUse">
          <stop offset="0%" style={{ stopColor: 'var(--palette-accent-text)' }} />
          <stop offset="50%" style={{ stopColor: 'var(--palette-accent-line)' }} />
          <stop offset="100%" style={{ stopColor: 'var(--palette-accent)' }} />
        </linearGradient>
      </defs>
      <path d="M6.5 3.5C15.5 4.2 22.8 11.4 23.2 21.5C14.6 21.2 7.4 15.1 6.5 3.5Z" fill="url(#leaf-grad)" />
      <path d="M6.5 3.5C12.8 9.4 17.5 15.2 21.5 23.8" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeOpacity="0.85" />
      <path d="M4.5 11.5C10.5 13.2 15.2 17.4 17.8 23.8C10.4 22.6 5.6 17.8 4.5 11.5Z" fill="var(--palette-accent)" fillOpacity="0.65" />
    </svg>
  )
}

export function WorkspaceSwitcher({ workspace, onSwitch }: {
  workspace: Workspace
  onSwitch: (next: Workspace) => void
}) {
  const current = WORKSPACES.find((w) => w.id === workspace) ?? WORKSPACES[0]!
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label="Workspace"
        className="group -mx-1.5 flex items-center gap-2.5 rounded-xl border border-transparent px-2 py-1.5 outline-none transition-all hover:border-border/70 hover:bg-sidebar-accent/80 focus-visible:ring-3 focus-visible:ring-ring/50 data-[popup-open]:border-border/80 data-[popup-open]:bg-sidebar-accent"
      >
        <span className="flex size-7 shrink-0 items-center justify-center rounded-lg border border-accent-line/40 bg-gradient-to-br from-accent/35 via-accent-softer to-background text-accent-foreground">
          <LeafMark className="size-4" />
        </span>
        {/* The wordmark labels the current workspace. */}
        <span className="bg-gradient-to-r from-foreground via-foreground to-accent-line bg-clip-text text-sm font-bold tracking-wider text-transparent">
          {current.label}
        </span>
        <ChevronsUpDown
          aria-hidden
          className="size-3.5 text-faint-foreground transition-colors group-hover:text-sidebar-foreground"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="min-w-64 border-border/90 bg-card/95 backdrop-blur-xl">
        <DropdownMenuRadioGroup
          value={workspace}
          onValueChange={(v) => { if (v !== workspace) onSwitch(v as Workspace) }}
        >
          {WORKSPACES.map((w) => (
            <DropdownMenuRadioItem key={w.id} value={w.id} className="py-2">
              <span className="flex flex-col gap-0.5">
                <span className="text-sm font-semibold">{w.label}</span>
                <span className="text-xs text-muted-foreground">{w.description}</span>
              </span>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
