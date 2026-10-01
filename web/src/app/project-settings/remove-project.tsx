// One job: removing the project (spec §4.2), the danger zone at the foot of
// its settings.
//
// Only a project with no conversation can go, and deleting a conversation is
// not built yet: so with any, Remove is off and says why. The daemon checks
// again, and a conversation started elsewhere in the meantime comes back as
// `PROJECT_HAS_THREADS`, shown in the dialog.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Trash2 } from 'lucide-react'
import { useRef, useState } from 'react'
import { type Project, removeProject } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { projectsQuery, threadsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorLine } from '../error-line'

export function RemoveProject({ project }: { project: Project }) {
  const threads = useQuery(threadsQuery(project.id)).data
  const [open, setOpen] = useState(false)
  const count = threads?.length

  return (
    <section className="space-y-3 rounded-md border border-destructive-border p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <h2 className="text-sm font-medium text-destructive-foreground">Remove project</h2>
          <p className="text-xs text-muted-foreground">
            {count !== undefined && count > 0
              ? count === 1
                ? 'This project has 1 conversation. Delete it first to remove the project.'
                : `This project has ${count} conversations. Delete them first to remove it.`
              : 'Hide this project from Shadows. Its folder on disk is not touched.'}
          </p>
        </div>
        <Button
          size="sm"
          variant="destructive"
          onClick={() => setOpen(true)}
          // Off until the conversations are known, and while there are any.
          disabled={count !== 0}
        >
          <Trash2 />
          Remove
        </Button>
      </div>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          {/* The popup unmounts when closed, so every opening starts afresh. */}
          <ConfirmRemove project={project} />
        </DialogContent>
      </Dialog>
    </section>
  )
}

function ConfirmRemove({ project }: { project: Project }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const pending = useRef<Attempt | null>(null)

  const remove = useMutation({
    mutationFn: (commandId: string) => removeProject(project.id, commandId),
    onSuccess: async () => {
      pending.current = null
      // Home first, so nothing on screen asks after the project any more.
      await navigate({ to: '/' })
      queryClient.setQueryData<Project[]>(projectsQuery.queryKey, (list) =>
        list?.filter((p) => p.id !== project.id),
      )
      void queryClient.invalidateQueries({ queryKey: projectsQuery.queryKey })
    },
    // A conversation started since: the zone learns it and turns Remove off.
    onError: () =>
      void queryClient.invalidateQueries({ queryKey: threadsQuery(project.id).queryKey }),
  })

  const confirm = () => {
    pending.current = attemptFor(pending.current, { remove: project.id })
    remove.mutate(pending.current.commandId)
  }

  return (
    <div className="grid gap-4">
      <DialogHeader>
        <DialogTitle>
          Remove <span dir="auto">{project.name}</span>?
        </DialogTitle>
        <DialogDescription>
          The project is hidden from Shadows, and its code index and links are dropped. Its folder
          on disk is not touched. Its slug, <code className="font-mono">{project.slug}</code>,
          stays taken.
        </DialogDescription>
      </DialogHeader>
      {remove.error !== null && <ErrorLine error={remove.error} />}
      <DialogFooter>
        <DialogClose render={<Button type="button" variant="outline" />}>Cancel</DialogClose>
        <Button variant="destructive" onClick={confirm} disabled={remove.isPending}>
          {remove.isPending ? 'Removing…' : 'Remove project'}
        </Button>
      </DialogFooter>
    </div>
  )
}
