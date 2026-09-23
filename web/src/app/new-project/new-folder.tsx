// One job: "New folder here" — asking for a name and creating that folder in
// the directory the browser shows.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { FolderPlus } from 'lucide-react'
import { useState } from 'react'
import { createDir } from '@/api/client'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

export function NewFolder({
  parent,
  onCreated,
}: {
  parent: string
  onCreated: (path: string) => void
}) {
  const queryClient = useQueryClient()
  const [name, setName] = useState<string | null>(null)
  const create = useMutation({
    mutationFn: (folder: string) => createDir(parent, folder),
    onSuccess: (entry) => {
      void queryClient.invalidateQueries({ queryKey: ['fs', 'dirs', parent] })
      setName(null)
      onCreated(entry.path)
    },
  })

  if (name === null) {
    return (
      <button
        type="button"
        onClick={() => {
          create.reset()
          setName('')
        }}
        className="flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <FolderPlus className="size-3.5" />
        New folder here
      </button>
    )
  }

  const submit = () => {
    if (name.trim() !== '') create.mutate(name.trim())
  }

  return (
    <div className="space-y-2 border-t border-border px-3 py-2">
      <div className="flex items-center gap-2">
        <Input
          autoFocus
          aria-label="New folder name"
          placeholder="Folder name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              // Enter here names the folder; it must not submit the dialog.
              e.preventDefault()
              submit()
            } else if (e.key === 'Escape') {
              e.preventDefault()
              e.stopPropagation()
              setName(null)
            }
          }}
          className="h-7 text-xs"
        />
        <Button type="button" size="sm" onClick={submit} disabled={create.isPending}>
          Create
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={() => setName(null)}>
          Cancel
        </Button>
      </div>
      {create.isError && <ErrorLine error={create.error} />}
    </div>
  )
}
