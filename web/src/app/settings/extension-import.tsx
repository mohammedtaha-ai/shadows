import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState, type FormEvent } from 'react'
import { addExtension, extensionsQuery, type ExtensionKind } from '@/api/extensions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

export function ExtensionImport({ kind, onDone }: { kind: ExtensionKind; onDone: () => void }) {
  const client = useQueryClient()
  const [path, setPath] = useState('')
  const [invalid, setInvalid] = useState<string | null>(null)
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const add = useMutation({ retry: false,
    mutationFn: (source: string) => {
      attempt.current = attemptFor(attempt.current, { kind, path: source })
      return addExtension(attempt.current.commandId, kind, source)
    },
    onSuccess: async () => {
      attempt.current = null; setPath('')
      await Promise.all([client.invalidateQueries({ queryKey: extensionsQuery.queryKey }), client.invalidateQueries({ queryKey: ['aylul-agent-extensions'] })])
      onDone()
    },
    onSettled: () => { locked.current = false },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (locked.current) return
    if (!path.trim()) return setInvalid('Enter a folder path.')
    setInvalid(null); locked.current = true; add.mutate(path.trim())
  }
  return <div className="space-y-4">
    <Button type="button" variant="ghost" aria-label="Back" disabled={add.isPending} onClick={onDone}>←</Button>
    <form onSubmit={submit} className="grid gap-4">
      <label className="grid gap-2 text-sm">Folder path<Input aria-label="Folder path" autoFocus value={path} disabled={add.isPending} onChange={event => setPath(event.target.value)} /></label>
      <p className="text-xs text-muted-foreground">Folder on the daemon's machine.</p>
      {invalid && <p role="alert">{invalid}</p>}
      {add.error && <ErrorLine error={add.error} />}
      <Button type="submit" disabled={add.isPending || !path.trim()}>{add.isPending ? 'Adding…' : 'Add'}</Button>
    </form>
  </div>
}
