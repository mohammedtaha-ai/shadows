// One job: owning whether the one New project dialog is open.

import { type ReactNode, useState } from 'react'
import { NewProjectDialog } from './new-project-dialog'
import { OpenNewProject } from './open-new-project'

export function NewProjectProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false)
  return (
    <OpenNewProject value={() => setOpen(true)}>
      {children}
      <NewProjectDialog open={open} onOpenChange={setOpen} />
    </OpenNewProject>
  )
}
