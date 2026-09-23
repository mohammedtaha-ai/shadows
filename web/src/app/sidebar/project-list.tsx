// One job: the sidebar's projects, each with its folder, and the open one's
// conversations under it.

import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from '@tanstack/react-router'
import { Folder, FolderOpen } from 'lucide-react'
import type { Project } from '@/api/client'
import { projectsQuery } from '@/api/queries'
import { ThreadList } from './thread-list'

export function ProjectList() {
  const { data: projects } = useQuery(projectsQuery)
  const { projectId, threadId } = useParams({ strict: false })

  if (projects === undefined) return null
  if (projects.length === 0) {
    return <p className="px-2 py-1 text-xs text-faint-foreground">No projects yet.</p>
  }

  return (
    <ul className="space-y-0.5">
      {projects.map((project) => (
        <li key={project.id}>
          <ProjectRow project={project} open={project.id === projectId} />
          {project.id === projectId && <ThreadList projectId={project.id} selected={threadId} />}
        </li>
      ))}
    </ul>
  )
}

function ProjectRow({ project, open }: { project: Project; open: boolean }) {
  const Icon = open ? FolderOpen : Folder
  return (
    <Link
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      className={`flex items-start gap-2.5 rounded-md px-2 py-1.5 transition-colors hover:bg-sidebar-accent ${
        open ? 'text-sidebar-foreground' : 'text-muted-foreground'
      }`}
    >
      <Icon className={`mt-0.5 size-4 shrink-0 ${open ? 'text-accent-line' : ''}`} />
      <span className="min-w-0">
        <span className="block truncate text-sm">{project.name}</span>
        {project.directory != null && (
          <span className="block truncate font-mono text-[11px] text-faint-foreground">
            {project.directory}
          </span>
        )}
      </span>
    </Link>
  )
}
