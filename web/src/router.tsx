// One job: the app's routes.
//
// Code-based rather than file-based: file-based routing needs the router's
// Vite plugin and a generated route tree, which earn their place when routes
// are many. There are five: nothing open, a project open, a conversation open,
// a plan version open, a project's settings — each a URL, so a reload
// restores what was open.

import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router'
import { ConversationRoute } from './app/conversation/conversation'
import { Home } from './app/home'
import { ProjectPage } from './app/project-page'
import { ProjectSettings } from './app/project-settings/project-settings'
import { Shell } from './app/shell'
import { PlanPage } from './app/workflows/plan-page'

const rootRoute = createRootRoute({ component: Shell })

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: Home,
})

const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$projectId',
  component: ProjectPage,
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
