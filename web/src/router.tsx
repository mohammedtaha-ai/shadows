// One job: the app's routes.
//
// Code-based rather than file-based: file-based routing needs the router's
// Vite plugin and a generated route tree, which earn their place when routes
// are many. There are three: nothing open, a project open, a conversation open
// — each a URL, so a reload restores what was open.

import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router'
import { ConversationRoute } from './app/conversation/conversation'
import { Home } from './app/home'
import { ProjectPage } from './app/project-page'
import { Shell } from './app/shell'

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

export const router = createRouter({
  routeTree: rootRoute.addChildren([homeRoute, projectRoute, threadRoute]),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
