// One job: which projects the sidebar shows open, remembered across reloads.

import { useEffect, useState } from 'react'

const KEY = 'shadows.sidebar.open-projects'

/** The stored set, or `null` when storage is missing, blocked or holds
 * anything but a list of ids. */
function read(): string[] | null {
  try {
    const raw = window.localStorage.getItem(KEY)
    if (raw === null) return null
    const ids: unknown = JSON.parse(raw)
    return Array.isArray(ids) && ids.every((id) => typeof id === 'string') ? ids : null
  } catch {
    return null
  }
}

function write(ids: ReadonlySet<string>): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify([...ids]))
  } catch {
    // Storage is a convenience: without it the set lasts until the reload.
  }
}

/** The open projects, and a toggle for one. The project in the URL opens
 * whenever the URL comes to it; closing it afterwards leaves the URL alone. */
export function useOpenProjects(urlProject: string | undefined) {
  const [open, setOpen] = useState<ReadonlySet<string>>(() => {
    const ids = new Set(read() ?? [])
    if (urlProject !== undefined) ids.add(urlProject)
    return ids
  })
  const [seen, setSeen] = useState(urlProject)
  if (seen !== urlProject) {
    // Adjusted while rendering, not in an effect: the section is open in the
    // same frame as the page it belongs to.
    setSeen(urlProject)
    if (urlProject !== undefined && !open.has(urlProject)) {
      setOpen(new Set(open).add(urlProject))
    }
  }

  useEffect(() => write(open), [open])

  const toggle = (projectId: string) =>
    setOpen((current) => {
      const next = new Set(current)
      if (!next.delete(projectId)) next.add(projectId)
      return next
    })

  return { isOpen: (projectId: string) => open.has(projectId), toggle }
}
