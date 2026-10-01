// One job: the project's code index (§15.6) — how its index stands, the
// projects whose code it reads, and linking or unlinking one.
//
// A link answers slugs only, so each row is joined with the project list for
// its name, folder and id; the routes that change a link take ids (§15.7).

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, X } from 'lucide-react'
import { useRef } from 'react'
import {
  type Project,
  type ProjectLink,
  type ProjectStatus,
  putCodeLink,
  removeCodeLink,
} from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { codeLinksQuery, codeStatusQuery, projectsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ErrorLine } from '../error-line'
import { ago } from './when'

export function CodeIndex({ project }: { project: Project }) {
  const status = useQuery(codeStatusQuery(project.id))
  const links = useQuery(codeLinksQuery(project.id))
  const projects = useQuery(projectsQuery).data ?? []

  const linked = new Set(links.data?.map((l) => l.linked))
  const candidates = projects.filter((p) => p.id !== project.id && !linked.has(p.slug))

  return (
    <section className="space-y-3">
      <div className="space-y-1">
        <h2 className="text-sm font-medium">Code index</h2>
        {status.data !== undefined && <IndexStatus status={status.data} />}
        {status.error !== null && <ErrorLine error={status.error} />}
      </div>
      <div className="space-y-2">
        <h3 className="text-xs text-muted-foreground">Reads code from</h3>
        {links.error !== null && <ErrorLine error={links.error} />}
        {links.data !== undefined && links.data.length === 0 && (
          <p className="text-xs text-faint-foreground">Only this project.</p>
        )}
        <ul className="divide-y divide-border rounded-md border border-border empty:hidden">
          {links.data?.map((l) => (
            <LinkedRow
              key={l.linked}
              link={l}
              linked={projects.find((p) => p.slug === l.linked)}
              projectId={project.id}
            />
          ))}
        </ul>
        <p className="text-xs text-faint-foreground">
          Questions asked in this project also search these projects. A link goes one way: they do
          not search this one.
        </p>
        <LinkPicker projectId={project.id} candidates={candidates} />
      </div>
    </section>
  )
}

/** `● Ready · 277 files · updated 2 min. ago`, coloured by the state. */
function IndexStatus({ status }: { status: ProjectStatus }) {
  const { state } = status
  const [dot, label] =
    state.state === 'ready'
      ? ['bg-success', 'Ready']
      : state.state === 'indexing'
        ? ['animate-pulse bg-accent-line', `Indexing ${state.done} of ${state.found}`]
        : state.state === 'inactive'
          ? ['bg-faint-foreground', 'Inactive']
          : state.state === 'no_directory'
            ? ['bg-destructive', 'No folder']
            : ['bg-destructive', 'Folder missing']
  const skipped = status.skipped.reduce((sum, s) => sum + s.count, 0)
  const parts = [
    label,
    state.state !== 'indexing' && `${status.files} files`,
    skipped > 0 && `${skipped} skipped`,
    status.updated_at != null && `updated ${ago(status.updated_at)}`,
  ].filter((part) => part !== false)

  return (
    <p
      className="flex items-center gap-2 text-xs text-muted-foreground"
      // The reasons a file was left out, for whoever asks.
      title={status.skipped.map((s) => `${s.count} ${s.reason}`).join('\n') || undefined}
    >
      <span aria-hidden className={`size-2 shrink-0 rounded-full ${dot}`} />
      <span>{parts.join(' · ')}</span>
    </p>
  )
}

/** One project this one reads: its name, folder and index, and × to unlink. */
function LinkedRow({
  link,
  linked,
  projectId,
}: {
  link: ProjectLink
  /** `undefined` while the project list has not answered. */
  linked: Project | undefined
  projectId: string
}) {
  const queryClient = useQueryClient()
  const status = useQuery({ ...codeStatusQuery(linked?.id ?? ''), enabled: linked !== undefined })
  const pending = useRef<Attempt | null>(null)
  const unlink = useMutation({
    mutationFn: ({ linkedId, commandId }: { linkedId: string; commandId: string }) =>
      removeCodeLink(projectId, linkedId, commandId),
    onSuccess: () => {
      pending.current = null
      queryClient.setQueryData<ProjectLink[]>(codeLinksQuery(projectId).queryKey, (list) =>
        list?.filter((l) => l.linked !== link.linked),
      )
      void queryClient.invalidateQueries({ queryKey: codeLinksQuery(projectId).queryKey })
    },
  })

  const startUnlink = () => {
    if (linked === undefined) return
    pending.current = attemptFor(pending.current, { unlink: linked.id })
    unlink.mutate({ linkedId: linked.id, commandId: pending.current.commandId })
  }

  const name = linked?.name ?? link.linked
  return (
    <li data-link={link.linked} className="space-y-1 px-3 py-2">
      <div className="flex items-start gap-3">
        <span className="min-w-0 flex-1 space-y-0.5">
          <span dir="auto" className="block truncate text-start text-sm">
            {name}
          </span>
          {linked?.directory != null && (
            <span className="block truncate font-mono text-[11px] text-faint-foreground">
              {linked.directory}
            </span>
          )}
          {status.data !== undefined && <IndexStatus status={status.data} />}
        </span>
        <Button
          size="icon-xs"
          variant="ghost"
          onClick={startUnlink}
          disabled={linked === undefined || unlink.isPending}
          aria-label={`Unlink ${name}`}
          title="Unlink"
        >
          <X />
        </Button>
      </div>
      {status.error !== null && <ErrorLine error={status.error} />}
      {unlink.error !== null && <ErrorLine error={unlink.error} />}
    </li>
  )
}

/** "Link a project…": the other projects, not yet linked, to choose one from. */
function LinkPicker({ projectId, candidates }: { projectId: string; candidates: Project[] }) {
  const queryClient = useQueryClient()
  const pending = useRef<Attempt | null>(null)
  const link = useMutation({
    mutationFn: ({ linkedId, commandId }: { linkedId: string; commandId: string }) =>
      putCodeLink(projectId, linkedId, commandId),
    onSuccess: () => {
      pending.current = null
      void queryClient.invalidateQueries({ queryKey: codeLinksQuery(projectId).queryKey })
    },
  })

  const pick = (linkedId: string) => {
    // Choosing the same project again after a lost answer is a retry.
    pending.current = attemptFor(pending.current, { link: linkedId })
    link.mutate({ linkedId, commandId: pending.current.commandId })
  }

  return (
    <div className="space-y-2">
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              size="sm"
              variant="secondary"
              disabled={candidates.length === 0 || link.isPending}
            />
          }
        >
          Link a project…
          <ChevronDown aria-hidden />
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          {candidates.map((p) => (
            <DropdownMenuItem key={p.id} onClick={() => pick(p.id)}>
              <span className="min-w-0">
                <span dir="auto" className="block truncate text-start">
                  {p.name}
                </span>
                {p.directory != null && (
                  <span className="block truncate font-mono text-[11px] text-faint-foreground">
                    {p.directory}
                  </span>
                )}
              </span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
      {link.error !== null && <ErrorLine error={link.error} />}
    </div>
  )
}
