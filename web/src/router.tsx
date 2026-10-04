// One job: the app's routes.
//
// Code-based rather than file-based: file-based routing needs the router's
// Vite plugin and a generated route tree, which earn their place when routes
// are many. Each open thing is a URL, so a reload restores it: nothing open, a
// new conversation's draft, a conversation, a plan version, a project's
// settings, the global settings. A project's own URL has no page: it opens
// the project's draft.

import { createRootRoute, createRoute, createRouter, redirect } from '@tanstack/react-router'
import { ConversationRoute } from './app/conversation/conversation'
import { DraftRoute } from './app/conversation/draft'
import { Home } from './app/home'
import { WorkspacePage } from './app/design/workspace-page'
import { ProjectSettings } from './app/project-settings/project-settings'
import { SettingsPage } from './app/settings/settings-page'
import { Shell } from './app/shell'
import { PlanPage } from './app/workflows/plan-page'

const rootRoute = createRootRoute({ component: Shell })

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: Home,
})

// Kept because links and older tabs name it; what a person opens a project
// for is to talk in it, and settings are one row away in the sidebar.
const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId',
  beforeLoad: ({ params }) => {
    throw redirect({ to: '/projects/$projectId/new', params, replace: true })
  },
})

const draftRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId/new',
  validateSearch: (search: Record<string, unknown>): { plan?: string } => ({
    plan: typeof search.plan === 'string' ? search.plan : undefined,
  }),
  component: DraftRoute,
})

const threadRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId/threads/$threadId',
  component: ConversationRoute,
})

const planRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId/workflows/$workflowId',
  component: PlanPage,
})

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId/settings',
  component: ProjectSettings,
})

const workspaceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId/workspace',
  validateSearch: (search: Record<string, unknown>): { view?: 'vision' | 'map' | 'roadmap' | 'plans'; part?: string; outcome?: string } => ({
    view: search.view === 'map' || search.view === 'roadmap' || search.view === 'plans' ? search.view : 'vision',
    part: typeof search.part === 'string' ? search.part : undefined,
    outcome: typeof search.outcome === 'string' ? search.outcome : undefined,
  }),
  component: WorkspacePage,
})

// The daemon's own settings, for every project at once (§13.11).
const appSettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})

export const router = createRouter({
  routeTree: rootRoute.addChildren([
    homeRoute,
    projectRoute,
    draftRoute,
    threadRoute,
    planRoute,
    settingsRoute,
    workspaceRoute,
    appSettingsRoute,
  ]),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
