// One job: the way any part of the app asks for the New project dialog. The
// sidebar's button and the empty state's invitation are two doors to one
// dialog, which `NewProjectProvider` renders.

import { createContext, use } from 'react'

export const OpenNewProject = createContext<(() => void) | null>(null)

/** Opens the New project dialog. */
export function useOpenNewProject(): () => void {
  const open = use(OpenNewProject)
  if (open === null) throw new Error('useOpenNewProject needs a NewProjectProvider')
  return open
}
