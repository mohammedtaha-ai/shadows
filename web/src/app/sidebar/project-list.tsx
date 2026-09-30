// One job: the sidebar's projects as a tree — each folds open to its
// conversations, plans and settings, and starts a new conversation from its row.

import { useQuery } from '@tanstack/react-query'
import { Link, useLocation, useParams } from '@tanstack/react-router'
import { ChevronRight, Folder, FolderOpen, Plus, Settings } from 'lucide-react'
import type { Project } from '@/api/client'
import { projectsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { useNewConversation } from '../use-new-conversation'
import { useOpenProjects } from './open-projects'
import { ThreadList } from './thread-list'
import { WorkflowList } from './workflow-list'

export function ProjectList() {
  const { data: projects } = useQuery(projectsQuery)
  const { projectId, threadId, workflowId } = useParams({ strict: false })
  const { isOpen, toggle } = useOpenProjects(projectId)

  if (projects === undefined) return null
  if (projects.length === 0) {
    return <p className="px-2 py-1 text-xs text-faint-foreground">No projects yet.</p>
  }

  return (
    <ul className="space-y-0.5">
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
                </>
              )}
            </div>
          </li>
        )
      })}
    </ul>
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
  const newConversation = useNewConversation(project.id)
  const Icon = open ? FolderOpen : Folder
  return (
    <>
      <div className="group relative flex items-start rounded-md transition-colors hover:bg-sidebar-accent">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          aria-controls={sectionId}
          className={`flex min-w-0 flex-1 items-start gap-1.5 rounded-md py-1.5 ps-1 pe-8 text-left ${
            open || current ? 'text-sidebar-foreground' : 'text-muted-foreground'
          }`}
        >
          <ChevronRight
            aria-hidden
            className={`mt-0.5 size-4 shrink-0 text-faint-foreground transition-transform ${open ? 'rotate-90' : ''}`}
          />
          <Icon
            aria-hidden
            className={`mt-0.5 size-4 shrink-0 ${current ? 'text-accent-line' : ''}`}
          />
          <span className="min-w-0 flex-1">
            <span dir="auto" className="block truncate text-start text-sm">
              {project.name}
            </span>
            {project.directory != null && (
              <span className="block truncate font-mono text-[11px] text-faint-foreground">
                {project.directory}
              </span>
            )}
          </span>
        </button>
        <button
          type="button"
          onClick={newConversation.start}
          disabled={newConversation.pending || !newConversation.ready}
          aria-label={`New conversation in ${project.name}`}
          title="New conversation"
          className="absolute top-1 right-1 rounded-md p-1 text-faint-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:bg-secondary hover:text-sidebar-foreground focus-visible:opacity-100 disabled:opacity-50"
        >
          <Plus aria-hidden className="size-3.5" />
        </button>
      </div>
      {newConversation.error !== null && (
        <div className="px-2 py-1">
          <ErrorLine error={newConversation.error} />
        </div>
      )}
    </>
  )
}

function SettingsLink({ projectId }: { projectId: string }) {
  const isSelected = useLocation().pathname === `/projects/${projectId}/settings`
  return (
    <div className="mb-1.5 ml-4 border-l border-sidebar-border pl-2">
      <Link
        to="/projects/$projectId/settings"
        params={{ projectId }}
        aria-current={isSelected ? 'page' : undefined}
        className={`flex items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-colors ${
          isSelected
            ? 'border-accent-line bg-secondary text-secondary-foreground'
            : 'border-transparent text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
        }`}
      >
        <Settings className="size-3.5 shrink-0" aria-hidden />
        Project settings
      </Link>
    </div>
  )
}
