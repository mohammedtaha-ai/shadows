// One job: the sidebar's projects as a tree — each folds open to its
// conversations, plans and settings, and opens a new conversation from its row.

import { useQuery } from '@tanstack/react-query'
import { Link, useLocation, useParams } from '@tanstack/react-router'
import { ChevronRight, Folder, FolderOpen, Layers, Plus, Settings } from 'lucide-react'
import type { Project } from '@/api/projects'
import { projectsQuery } from '@/api/projects'
import { useOpenProjects } from './open-projects'
import { ThreadList } from './thread-list'
import { WorkflowList } from './workflow-list'

export function ProjectList({ filter = '' }: { filter?: string }) {
  // The hidden assistant project belongs to the AYLUL workspace.
  const allProjects = useQuery(projectsQuery).data?.filter((p) => !p.assistant)
  const q = filter.trim().toLowerCase()
  const projects = allProjects === undefined || q === ''
    ? allProjects
    : allProjects.filter((p) => p.name.toLowerCase().includes(q) || (p.directory?.toLowerCase().includes(q) ?? false))
  const { projectId, threadId, workflowId } = useParams({ strict: false })
  const { isOpen, toggle } = useOpenProjects(projectId)

  if (allProjects === undefined || projects === undefined) return null
  if (allProjects.length === 0) {
    return <EmptyLine>No projects yet.</EmptyLine>
  }
  if (projects.length === 0) {
    return <EmptyLine>No matching projects.</EmptyLine>
  }

  return (
    <ul className="space-y-1">
      {projects.map((project) => {
        const open = isOpen(project.id)
        const sectionId = `project-${project.id}-section`
        return (
          <li key={project.id}>
            <ProjectRow
              project={project}
              open={open}
              current={project.id === projectId}
              sectionId={sectionId}
              onToggle={() => toggle(project.id)}
            />
            {/* Always present, so `aria-controls` names an element; filled
                only while open, so a folded project reads nothing. */}
            <div id={sectionId}>
              {open && (
                <>
                  <ThreadList
                    projectId={project.id}
                    selected={project.id === projectId ? threadId : undefined}
                  />
                  <WorkflowList
                    projectId={project.id}
                    selected={project.id === projectId ? workflowId : undefined}
                  />
                  <SettingsLink projectId={project.id} />
                  <Link to="/projects/$projectId/workspace" params={{ projectId: project.id }}
                    className="ml-6 flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-xs text-muted-foreground transition-all hover:bg-sidebar-accent/70 hover:text-sidebar-foreground"
                    activeProps={{ 'aria-current': 'page', className: 'pill-active font-medium text-sidebar-foreground' }}>
                    <Layers className="size-3.5 shrink-0 text-accent-line/80" aria-hidden />
                    Workspace
                  </Link>
                </>
              )}
            </div>
          </li>
        )
      })}
    </ul>
  )
}

function EmptyLine({ children }: { children: string }) {
  return (
    <div className="rounded-xl border border-border/60 bg-card/40 px-3 py-2.5">
      <p className="text-xs text-faint-foreground">{children}</p>
    </div>
  )
}

/** The row folds its project open or closed and never navigates, so folding
 * the project that holds the open conversation keeps that conversation. */
function ProjectRow({
  project,
  open,
  current,
  sectionId,
  onToggle,
}: {
  project: Project
  open: boolean
  /** The project the URL is in. */
  current: boolean
  sectionId: string
  onToggle: () => void
}) {
  const Icon = open ? FolderOpen : Folder
  return (
    <div
      className={`group relative flex items-start rounded-xl border transition-all ${
        current
          ? 'border-accent-line/45 bg-gradient-to-r from-accent/25 via-accent-softer/90 to-transparent'
          : open
            ? 'border-border/70 bg-sidebar-accent/70'
            : 'border-transparent hover:border-border/50 hover:bg-sidebar-accent/50'
      }`}
    >
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          aria-controls={sectionId}
          className={`flex min-w-0 flex-1 items-start gap-2 rounded-xl py-2 ps-2 pe-8 text-left ${
            open || current ? 'font-medium text-sidebar-foreground' : 'text-muted-foreground'
          }`}
        >
          <ChevronRight
            aria-hidden
            className={`mt-0.5 size-4 shrink-0 text-faint-foreground transition-transform duration-150 ${open ? 'rotate-90 text-accent-line' : ''}`}
          />
          <Icon
            aria-hidden
            className={`mt-0.5 size-4 shrink-0 ${current ? 'text-accent-line' : 'text-muted-foreground'}`}
          />
          <span className="min-w-0 flex-1">
            <span dir="auto" className="block truncate text-start text-sm">
              {project.name}
            </span>
            {project.directory != null && (
              <span className="block truncate font-mono text-[11px] font-normal text-faint-foreground">
                {project.directory}
              </span>
            )}
          </span>
        </button>
        {/* The only way to add a conversation: it opens a draft, and the
            conversation exists once its first message is sent. */}
        <Link
          to="/projects/$projectId/new"
          params={{ projectId: project.id }}
          aria-label={`New conversation in ${project.name}`}
          title="New conversation"
          className="absolute top-1.5 right-1.5 rounded-lg p-1 text-faint-foreground opacity-0 transition-all group-hover:opacity-100 hover:bg-accent/30 hover:text-sidebar-foreground focus-visible:opacity-100 pointer-coarse:opacity-100"
        >
          <Plus aria-hidden className="size-3.5" />
        </Link>
    </div>
  )
}

function SettingsLink({ projectId }: { projectId: string }) {
  const isSelected = useLocation().pathname === `/projects/${projectId}/settings`
  return (
    <div className="mb-1 ml-4 border-l border-sidebar-border/80 pl-2">
      <Link
        to="/projects/$projectId/settings"
        params={{ projectId }}
        aria-current={isSelected ? 'page' : undefined}
        className={`flex items-center gap-2 rounded-r-lg border-l-2 px-2.5 py-1.5 text-xs transition-all ${
          isSelected
            ? 'border-accent-line bg-secondary/90 font-medium text-secondary-foreground'
            : 'border-transparent text-muted-foreground hover:bg-sidebar-accent/70 hover:text-sidebar-foreground'
        }`}
      >
        <Settings className="size-3.5 shrink-0" aria-hidden />
        Project settings
      </Link>
    </div>
  )
}
