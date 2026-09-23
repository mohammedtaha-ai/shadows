// One job: the app's routes.
//
// Code-based rather than file-based: file-based routing needs the router's
// Vite plugin and a generated route tree, which earn their place when routes
// are many. There is one today.

import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router'
import { Home } from './app/home'
import { Shell } from './app/shell'

const rootRoute = createRootRoute({ component: Shell })

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: Home,
})

export const router = createRouter({ routeTree: rootRoute.addChildren([homeRoute]) })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
