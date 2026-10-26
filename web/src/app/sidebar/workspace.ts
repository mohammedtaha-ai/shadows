// One job: which workspace the sidebar shows — the page decides first, then
// the choice remembered in this browser.
import { useQuery } from '@tanstack/react-query'
import { useLocation, useNavigate } from '@tanstack/react-router'
import { useEffect, useSyncExternalStore } from 'react'
import { threadQuery } from '@/api/threads'
export type Workspace = 'shadows' | 'aylul'
let fallback: Workspace = 'shadows'
const listeners = new Set<() => void>()
export function storedWorkspace(): Workspace {
  try { return localStorage.getItem('shadows.workspace') === 'aylul' ? 'aylul' : 'shadows' }
  catch { return fallback }
}
function remember(workspace: Workspace) {
  fallback = workspace
  try { localStorage.setItem('shadows.workspace', workspace) } catch { /* In-memory fallback. */ }
  for (const listener of listeners) listener()
}
function subscribe(listener: () => void) {
  listeners.add(listener)
  window.addEventListener('storage', listener)
  return () => { listeners.delete(listener); window.removeEventListener('storage', listener) }
}
function lastPage(workspace: Workspace): string {
  try {
    const path = localStorage.getItem('shadows.workspace.last.' + workspace)
    if (path?.startsWith('/') && !path.startsWith('//') &&
      (workspace === 'aylul' ? path.startsWith('/aylul') || path.includes('/threads/')
        : !path.startsWith('/aylul'))) return path
  } catch { /* Optional browser storage. */ }
  return workspace === 'aylul' ? '/aylul' : '/'
}
export function useWorkspace() {
  const path = useLocation().pathname
  const navigate = useNavigate()
  const picked = useSyncExternalStore(subscribe, storedWorkspace, () => 'shadows' as Workspace)
  const threadId = path.match(/^\/projects\/[^/]+\/threads\/([^/]+)$/)?.[1]
  const thread = useQuery({ ...threadQuery(threadId ?? ''), enabled: threadId !== undefined })
  const forced: Workspace | undefined = path.startsWith('/aylul') ? 'aylul'
    : threadId !== undefined ? thread.data === undefined ? undefined
      : thread.data.harness === 'aylul' ? 'aylul' : 'shadows'
      : path.startsWith('/projects/') ? 'shadows' : undefined
  const workspace = forced ?? picked
  useEffect(() => {
    remember(workspace)
    try {
      if (threadId === undefined || thread.data !== undefined) {
        localStorage.setItem('shadows.workspace.last.' + workspace, path)
      }
    } catch { /* The in-memory preference still works. */ }
  }, [workspace, path, threadId, thread.data])
  const switchTo = (next: Workspace) => {
    remember(next)
    void navigate({ to: lastPage(next) })
  }
  return { workspace, switchTo }
}
