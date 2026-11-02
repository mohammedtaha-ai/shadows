import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { extensionsQuery, extensionKeys, removeExtension, type ExtensionKind, type Item, type McpItem } from '@/api/extensions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { ConfirmPanel } from '../confirm-panel'

export function ExtensionList({ kind, items, onEdit }: { kind: ExtensionKind; items: (Item | McpItem)[]; onEdit?: (item: McpItem) => void }) {
  const client = useQueryClient()
  const [confirming, setConfirming] = useState<{ kind: ExtensionKind; id: string } | null>(null)
  const attempts = useRef(new Map<string, Attempt>())
  const locked = useRef(false)
  const remove = useMutation({ retry: false,
    mutationFn: ({ kind, id }: { kind: ExtensionKind; id: string }) => {
      const key = `${kind}:${id}`
      const attempt = attemptFor(attempts.current.get(key) ?? null, { kind, id })
      attempts.current.set(key, attempt)
      return removeExtension(attempt.commandId, kind, id)
    },
    onSuccess: async (_, input) => {
      attempts.current.delete(`${input.kind}:${input.id}`)
      await Promise.all([client.invalidateQueries({ queryKey: extensionsQuery.queryKey }), client.invalidateQueries({ queryKey: extensionKeys.agents() })])
      setConfirming(null)
    },
    onSettled: () => { locked.current = false },
  })
  if (!items.length) return <p>{kind === 'plugin' ? 'No plugins yet.' : kind === 'skill' ? 'No skills yet.' : 'No MCP tools yet.'}</p>
  return <ul className="space-y-3">{items.map(item => <li key={item.id} className="surface-panel space-y-3 rounded-xl p-3">
    <div className="flex items-center gap-3"><div className="min-w-0 flex-1"><h3 className="font-medium">{item.name}</h3><p className="truncate text-sm text-muted-foreground" title={item.description}>{item.description}</p>{'bundled' in item && item.bundled && <p className="text-xs text-muted-foreground">Included with AYLUL</p>}</div>
      {kind === 'mcp' && 'transport' in item && <Button type="button" variant="outline" disabled={remove.isPending} onClick={() => onEdit?.(item)}>Edit</Button>}
      <Button type="button" variant="outline" disabled={remove.isPending || confirming?.kind === kind && confirming.id === item.id} onClick={() => { remove.reset(); setConfirming({ kind, id: item.id }) }}>Remove</Button>
    </div>
    {confirming?.kind === kind && confirming.id === item.id && <ConfirmPanel
      question={<>Remove {item.name} from the library and from every agent?</>}
      error={remove.error} confirmLabel="Remove" pending={remove.isPending}
      cancelDisabledWhilePending onCancel={() => setConfirming(null)} onConfirm={() => {
        if (locked.current) return
        locked.current = true; remove.mutate({ kind, id: item.id })
      }}
      presentation={{ kind: 'inline', ariaLabel: `Remove ${item.name}`, className: 'space-y-3 rounded-lg border border-destructive-border p-3' }}
    />}
  </li>)}</ul>
}
