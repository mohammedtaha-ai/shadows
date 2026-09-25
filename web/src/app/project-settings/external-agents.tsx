// One job: the external agents allowed on this project (§13.7) — Connect,
// the command it answers once, and the grants with Revoke.
//
// The token lives only in the one answer to Connect, so the command is kept in
// this component's state and nowhere else: leaving the page loses it, as the
// daemon's own copy was never stored. Only project grants are shown; a
// Planner's grant belongs to its session and is not the person's to manage.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { PlugZap } from 'lucide-react'
import { useRef, useState } from 'react'
import { type Grant, issueGrant, revokeGrant } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { grantsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { CopyButton } from '../copy-button'
import { ErrorLine } from '../error-line'
import { when } from './when'

export function ExternalAgents({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  // Polled every 10 s while shown (see `grantsQuery`).
  const grants = useQuery(grantsQuery(projectId))
  // What the last Connect answered: its command, or `null` for a replay that
  // no longer has one; `undefined` before any.
  const [issued, setIssued] = useState<string | null | undefined>(undefined)
  const pending = useRef<Attempt | null>(null)

  const connect = useMutation({
    mutationFn: (commandId: string) => issueGrant(projectId, commandId),
    // The answer holds the token: dropped from the shared mutation cache as
    // soon as this page no longer watches it, so only `issued` ever has it.
    gcTime: 0,
    onSuccess: (answer) => {
      pending.current = null
      setIssued(answer.token != null ? (answer.command ?? null) : null)
      void queryClient.invalidateQueries({ queryKey: grantsQuery(projectId).queryKey })
    },
  })

  const startConnect = () => {
    // A retry after a lost answer reuses the id, and the daemon answers the
    // replay without its token: the case `Issued` explains.
    pending.current = attemptFor(pending.current, { connect: projectId })
    connect.mutate(pending.current.commandId)
  }

  const shown = grants.data?.filter((g) => g.kind === 'project')

  return (
    <section className="space-y-3">
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <h2 className="text-sm font-medium">External agents</h2>
          <p className="text-xs text-muted-foreground">
            Let a Claude Code session outside Shadows read and edit this project&apos;s plans.
          </p>
        </div>
        <Button size="sm" variant="secondary" onClick={startConnect} disabled={connect.isPending}>
          <PlugZap />
          Connect
        </Button>
      </div>
      {connect.error !== null && <ErrorLine error={connect.error} />}
      {issued !== undefined && <Issued command={issued} />}
      {grants.error !== null && <ErrorLine error={grants.error} />}
      {shown !== undefined && shown.length === 0 && (
        <p className="text-xs text-faint-foreground">No external agent is connected.</p>
      )}
      <ul className="divide-y divide-border rounded-md border border-border empty:hidden">
        {shown?.map((g) => <GrantRow key={g.id} grant={g} projectId={projectId} />)}
      </ul>
    </section>
  )
}

/** Connect's answer: the command to copy, or why a replay has none. */
function Issued({ command }: { command: string | null }) {
  if (command === null) {
    return (
      <p className="rounded-md border border-border bg-muted px-3 py-2 text-xs text-muted-foreground">
        This connection was already created; its command is no longer shown. Revoke it and connect
        again if you need it.
      </p>
    )
  }

  return (
    <div className="space-y-2 rounded-md border border-accent-line bg-accent-softer p-3">
      <p className="text-xs text-secondary-foreground">
        Run this once in a terminal. It is shown only now.
      </p>
      <div className="flex items-start gap-2">
        <pre className="min-w-0 flex-1 overflow-x-auto rounded bg-background px-2 py-1.5 font-mono text-xs">
          {command}
        </pre>
        <CopyButton text={command} size="sm" />
      </div>
      <p className="text-xs text-muted-foreground">
        Claude Code stores this token in plain text in ~/.claude.json. Revoking it here stops
        Shadows accepting it; it does not remove it from Claude&apos;s settings.
      </p>
    </div>
  )
}

function GrantRow({ grant, projectId }: { grant: Grant; projectId: string }) {
  const queryClient = useQueryClient()
  const pending = useRef<Attempt | null>(null)
  const revoke = useMutation({
    mutationFn: (commandId: string) => revokeGrant(grant.id, commandId),
    onSuccess: (revoked) => {
      pending.current = null
      queryClient.setQueryData<Grant[]>(grantsQuery(projectId).queryKey, (list) =>
        list?.map((g) => (g.id === revoked.id ? revoked : g)),
      )
    },
  })

  const startRevoke = () => {
    pending.current = attemptFor(pending.current, { revoke: grant.id })
    revoke.mutate(pending.current.commandId)
  }

  const revokedAt = grant.revoked_at ?? null

  return (
    <li data-grant={grant.id} className="space-y-1 px-3 py-2">
      <div className="flex items-center gap-3 text-sm">
        <span className="min-w-0 flex-1">
          <span className="block">Connected {when(grant.created_at)}</span>
          <span className="block truncate font-mono text-[11px] text-faint-foreground">
            {grant.id}
          </span>
        </span>
        {revokedAt === null ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={startRevoke}
            disabled={revoke.isPending}
            // Each row's button says which connection it ends, by what the row shows.
            aria-label={`Revoke the connection made ${when(grant.created_at)}, ${grant.id}`}
          >
            Revoke
          </Button>
        ) : (
          <span className="shrink-0 text-xs text-faint-foreground">Revoked {when(revokedAt)}</span>
        )}
      </div>
      {revoke.error !== null && <ErrorLine error={revoke.error} />}
    </li>
  )
}
