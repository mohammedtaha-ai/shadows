// One job: the app's routes.
//
// Code-based rather than file-based: file-based routing needs the router's
// Vite plugin and a generated route tree, which earn their place when routes
// are many. Each open thing is a URL, so a reload restores it: nothing open, a
// new conversation's draft, a conversation, a plan version, a project's
// settings. A project's own URL has no page: it opens the project's draft.

import { createRootRoute, createRoute, createRouter, redirect } from '@tanstack/react-router'
import { ConversationRoute } from './app/conversation/conversation'
import { DraftRoute } from './app/conversation/draft'
import { Home } from './app/home'
import { ProjectSettings } from './app/project-settings/project-settings'
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

export const router = createRouter({
  routeTree: rootRoute.addChildren([
    homeRoute,
    projectRoute,
    draftRoute,
    threadRoute,
    planRoute,
    settingsRoute,
  ]),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
