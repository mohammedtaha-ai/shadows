// One job: the main pane when no project is open.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { ArrowUpRight, Compass, FolderGit2, FolderPlus, Sparkles } from 'lucide-react'
import { projectsQuery } from '@/api/projects'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { DaemonProblem } from './daemon-status'
import { useOpenNewProject } from '@/app/new-project'

export function Home() {
  const { status, data, error, refetch, isFetching } = useQuery(projectsQuery)
  const openNewProject = useOpenNewProject()

  if (status === 'error') {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <DaemonProblem error={error} retry={() => void refetch()} retrying={isFetching} />
      </div>
    )
  }

  if (status === 'success' && data.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
        <Panel size="lg" className="flex max-w-md flex-col items-center gap-4">
          <div className="flex size-14 items-center justify-center rounded-2xl border border-accent-line/40 bg-accent-line/15">
            <FolderPlus aria-hidden className="size-7 text-accent-line" />
          </div>
          <div className="space-y-1.5">
            <h1 className="text-base font-semibold text-foreground">Create your first project</h1>
            <p className="max-w-sm text-sm text-muted-foreground">
              A project is a folder on this machine. The Planner reads it and runs there.
            </p>
          </div>
          <Button onClick={openNewProject} className="pill-active">New project</Button>
        </Panel>
      </div>
    )
  }

  if (status === 'pending') {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <p className="text-sm text-faint-foreground">Looking for the daemon…</p>
      </div>
    )
  }

  // The assistant project backs the AYLUL workspace; it is not a folder to open here.
  const visibleProjects = data.filter((project) => !project.assistant)

  return (
    <div className="flex flex-1 flex-col overflow-y-auto px-8 py-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <Panel size="lg" className="relative overflow-hidden">
          <div aria-hidden className="pointer-events-none absolute -top-20 -right-20 size-64 rounded-full bg-accent-line/15 blur-3xl" />
          <div className="relative flex items-start gap-4">
            <div className="flex size-12 shrink-0 items-center justify-center rounded-2xl border border-accent-line/40 bg-accent-line/15">
              <FolderGit2 aria-hidden className="size-6 text-accent-line" />
            </div>
            <div className="space-y-1">
              <span className="inline-flex items-center gap-1 rounded-full border border-accent-line/35 bg-accent-line/15 px-2.5 py-0.5 text-[11px] font-semibold tracking-wider text-accent-line uppercase">
                <Sparkles aria-hidden className="size-3" />
                NYLRA · Shadows Planner
              </span>
              <h1 className="text-lg font-semibold tracking-tight text-foreground">Projects &amp; Architecture</h1>
              <p className="text-xs text-muted-foreground">Choose a project.</p>
            </div>
          </div>
        </Panel>

        {visibleProjects.length > 0 && (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {visibleProjects.map((project) => (
              <Panel key={project.id} className="flex flex-col justify-between gap-4 p-5 transition-all hover:border-accent-line/45">
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between gap-2">
                    <h2 dir="auto" className="truncate text-sm font-semibold text-foreground">{project.name}</h2>
                    <span className="rounded-md border border-accent-line/25 bg-accent-line/10 px-2 py-0.5 text-[10px] font-medium text-accent-line">
                      Workspace
                    </span>
                  </div>
                  {project.directory && (
                    <p dir="auto" className="truncate font-mono text-xs text-muted-foreground">{project.directory}</p>
                  )}
                </div>
                <div className="flex items-center gap-2 border-t border-border pt-3">
                  <Link
                    to="/projects/$projectId/new"
                    params={{ projectId: project.id }}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-accent-line/35 bg-accent-line/15 px-3 py-1.5 text-xs font-medium text-foreground transition-colors hover:bg-accent-line/30"
                  >
                    <span>Open Planner</span>
                    <ArrowUpRight aria-hidden className="size-3.5" />
                  </Link>
                  <Link
                    to="/projects/$projectId/workspace"
                    params={{ projectId: project.id }}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-input-background px-3 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:border-accent-line/35 hover:text-foreground"
                  >
                    <Compass aria-hidden className="size-3.5 text-accent-line" />
                    <span>Design &amp; Map</span>
                  </Link>
                </div>
              </Panel>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
