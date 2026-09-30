// One job: the New project dialog — a name, a folder, and the create call.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { type FormEvent, useReducer, useRef, useState } from 'react'
import { createProject } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { projectsQuery } from '@/api/queries'
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
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'
import { FolderBrowser } from './folder-browser'
import { chosenFolder, folderReducer, initialFolderState } from './folder-state'
import { slugify } from './slug'

export function NewProjectDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {/* The popup unmounts when closed, so every opening starts blank. */}
        <NewProjectForm onCreated={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

function NewProjectForm({ onCreated }: { onCreated: () => void }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [name, setName] = useState('')
  const [folder, dispatch] = useReducer(folderReducer, initialFolderState)
  // The unfinished attempt: pressing Create again with the same request is a
  // retry and reuses its command id (see `attemptFor`).
  const attempt = useRef<Attempt | null>(null)

  const create = useMutation({
    mutationFn: createProject,
    onSuccess: (project) => {
      attempt.current = null
      void queryClient.invalidateQueries({ queryKey: projectsQuery.queryKey })
      onCreated()
      void navigate({ to: '/projects/$projectId/new', params: { projectId: project.id } })
    },
  })

  const slug = slugify(name)
  const directory = chosenFolder(folder)
  const ready = slug !== '' && directory !== null && !create.isPending

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!ready) return
    const request = { name: name.trim(), slug, directory }
    attempt.current = attemptFor(attempt.current, request)
    create.mutate({ ...request, command_id: attempt.current.commandId })
  }

  return (
    <form onSubmit={submit} className="grid gap-4">
      <DialogHeader>
        <DialogTitle>New project</DialogTitle>
        <DialogDescription>
          A project owns one folder on this machine. Its conversations run there.
        </DialogDescription>
      </DialogHeader>

      <label className="grid gap-1.5">
        <span className="text-xs font-medium text-muted-foreground">Name</span>
        <Input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="My project"
        />
        <span className="font-mono text-[11px] text-faint-foreground">
          {slug === '' ? 'The slug comes from the name.' : `slug: ${slug}`}
        </span>
      </label>

      <div className="grid gap-1.5">
        <label htmlFor="new-project-folder" className="text-xs font-medium text-muted-foreground">
          Folder
        </label>
        <Input
          id="new-project-folder"
          value={folder.input}
          onChange={(e) => dispatch({ type: 'type', text: e.target.value })}
          onKeyDown={(e) => {
            // Enter here browses to the typed path instead of submitting.
            if (e.key === 'Enter') {
              e.preventDefault()
              dispatch({ type: 'go' })
            }
          }}
          placeholder="Type or paste a path, or choose below"
          className="font-mono text-xs"
          spellCheck={false}
        />
        <FolderBrowser state={folder} dispatch={dispatch} />
      </div>

      {create.isError && <ErrorLine error={create.error} />}

      <DialogFooter>
        <DialogClose render={<Button type="button" variant="outline" />}>Cancel</DialogClose>
        <Button type="submit" disabled={!ready}>
          {create.isPending ? 'Creating…' : 'Create project'}
        </Button>
      </DialogFooter>
    </form>
  )
}
