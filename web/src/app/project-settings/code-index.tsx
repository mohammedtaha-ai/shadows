// A link answers slugs only, so each row is joined with the project list for
// its name, folder and id; the routes that change a link take ids.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, X } from 'lucide-react'
import { useRef } from 'react'
import { type ProjectLink, putCodeLink, removeCodeLink } from '@/api/code'
import { type Project } from '@/api/projects'
import { type Attempt, attemptFor } from '@/api/command-id'
import { codeLinksQuery } from '@/api/code'
import { projectsQuery } from '@/api/projects'
import { saveUnlinkedCodeAnswer } from '@/api/code'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ErrorLine } from '../error-line'

export function CodeIndex({ project }: { project: Project }) {
  const links = useQuery(codeLinksQuery(project.id))
  const projects = useQuery(projectsQuery).data ?? []

  const linked = new Set(links.data?.map((l) => l.linked))
  const candidates = projects.filter((p) => p.id !== project.id && !linked.has(p.slug))

  return (
    <section className="space-y-3">
      <div className="space-y-2">
        <h2 className="text-sm font-medium">Reads code from</h2>
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

/** One project this one reads: its name and folder, and × to unlink. */
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
  const pending = useRef<Attempt | null>(null)
  const unlink = useMutation({
    mutationFn: ({ linkedId, commandId }: { linkedId: string; commandId: string }) =>
      removeCodeLink(projectId, linkedId, commandId),
    onSuccess: async () => {
      pending.current = null
      await saveUnlinkedCodeAnswer(queryClient, projectId, link.linked)
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
