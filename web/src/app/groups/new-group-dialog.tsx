import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import type { Agent } from '@/api/aylul-profiles'
import { attemptFor, type Attempt } from '@/api/command-id'
import { createGroup, groupKeys } from '@/api/groups'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { ErrorLine } from '../error-line'

export function NewGroupDialog({ agents, onClose, onCreated }: {
  agents: readonly Agent[]
  onClose: () => void
  onCreated: (id: string) => void
}) {
  const client = useQueryClient()
  const [name, setName] = useState('')
  const [members, setMembers] = useState<string[]>([])
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const create = useMutation({
    mutationFn: () => {
      const body = { name: name.trim(), members: [...members].sort() }
      attempt.current = attemptFor(attempt.current, body)
      return createGroup({ ...body, command_id: attempt.current.commandId })
    },
    onSuccess: group => {
      attempt.current = null
      client.setQueryData(groupKeys.detail(group.id), group)
      void client.invalidateQueries({ queryKey: groupKeys.list })
      onCreated(group.id)
    },
    onSettled: () => { locked.current = false },
  })
  const valid = name.trim() !== '' && members.length > 0 && members.every(slug => agents.some(agent => agent.slug === slug))
  return (
    <Dialog open onOpenChange={open => { if (!open && !locked.current) onClose() }}>
      <DialogContent>
        <DialogTitle>New group</DialogTitle>
        <DialogDescription>Choose up to six agents.</DialogDescription>
        <form className="space-y-4" onSubmit={event => {
          event.preventDefault()
          if (!valid || locked.current) return
          locked.current = true
          create.mutate()
        }}>
          <label htmlFor="new-group-name" className="block">Group name</label>
          <input id="new-group-name" value={name} maxLength={120} disabled={create.isPending}
            onChange={event => setName(event.target.value)}
            className="w-full rounded-md border border-border bg-input px-3 py-2" />
          <fieldset className="max-h-56 space-y-2 overflow-y-auto" disabled={create.isPending}>
            <legend className="mb-1">Members</legend>
            {agents.map(agent => {
              const checked = members.includes(agent.slug)
              return <label key={agent.slug} className="flex items-center gap-2">
                <input type="checkbox" checked={checked} disabled={create.isPending || (!checked && members.length === 6)}
                  onChange={() => setMembers(current => checked ? current.filter(slug => slug !== agent.slug) : [...current, agent.slug])} />
                <span dir="auto">{agent.name}</span>
              </label>
            })}
          </fieldset>
          {create.error && <ErrorLine error={create.error} />}
          <Button type="submit" disabled={!valid || create.isPending}>Create group</Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
