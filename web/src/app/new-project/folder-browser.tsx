// One job: showing one directory of the daemon's disk to choose a project
// folder from — its path, an "up" row, its subdirectories — and the way to
// create a new one there.

import { useQuery } from '@tanstack/react-query'
import { CornerLeftUp, Folder, HardDrive, LoaderCircle } from 'lucide-react'
import type { Dispatch, ReactNode } from 'react'
import { dirsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import type { FolderAction, FolderState } from './folder-state'
import { NewFolder } from './new-folder'

export function FolderBrowser({
  state,
  dispatch,
}: {
  state: FolderState
  dispatch: Dispatch<FolderAction>
}) {
  const listing = useQuery(dirsQuery(state.at))
  const chosen = state.input.trim()
  const current = listing.data?.path ?? state.at

  return (
    <div className="overflow-hidden rounded-lg border border-border bg-input-background">
      <div
        className={`flex items-center gap-2 border-b border-border px-3 py-2 font-mono text-xs ${
          current !== null && current === chosen
            ? 'bg-secondary text-secondary-foreground'
            : 'text-muted-foreground'
        }`}
      >
        {current === null ? <HardDrive className="size-3.5" /> : <Folder className="size-3.5" />}
        <span className="truncate">{current ?? 'This computer'}</span>
      </div>

      <ul className="h-56 overflow-y-auto py-1 text-sm" aria-label="Subfolders">
        {state.at !== null && (
          <Row onClick={() => dispatch({ type: 'up', parent: listing.data?.parent ?? null })}>
            <CornerLeftUp className="size-3.5 text-faint-foreground" />
            <span className="text-muted-foreground">Up</span>
          </Row>
        )}
        {listing.status === 'pending' && (
          <li className="flex items-center gap-2 px-3 py-1.5 text-xs text-faint-foreground">
            <LoaderCircle className="size-3.5 animate-spin motion-reduce:animate-none" />
            Reading…
          </li>
        )}
        {listing.status === 'error' && (
          <li className="px-3 py-1.5">
            <ErrorLine error={listing.error} />
          </li>
        )}
        {listing.data?.entries.map((entry) => (
          <Row
            key={entry.path}
            selected={entry.path === chosen}
            onClick={() => dispatch({ type: 'enter', path: entry.path })}
          >
            {state.at === null ? (
              <HardDrive className="size-3.5 text-faint-foreground" />
            ) : (
              <Folder className="size-3.5 text-faint-foreground" />
            )}
            <span className={`truncate ${entry.hidden ? 'text-faint-foreground' : ''}`}>
              {entry.name}
            </span>
          </Row>
        ))}
        {listing.status === 'success' && listing.data.entries.length === 0 && (
          <li className="px-3 py-1.5 text-xs text-faint-foreground">No subfolders.</li>
        )}
      </ul>

      {listing.status === 'success' && current !== null && (
        <NewFolder parent={current} onCreated={(path) => dispatch({ type: 'enter', path })} />
      )}
    </div>
  )
}

function Row({
  children,
  onClick,
  selected = false,
}: {
  children: ReactNode
  onClick: () => void
  selected?: boolean
}) {
  return (
    <li>
      <button
        type="button"
        onClick={onClick}
        className={`flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors hover:bg-muted ${
          selected ? 'bg-secondary text-secondary-foreground' : ''
        }`}
      >
        {children}
      </button>
    </li>
  )
}
