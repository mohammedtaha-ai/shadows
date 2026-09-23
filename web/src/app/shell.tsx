// One job: the two-pane frame every screen sits in.

import { Outlet } from '@tanstack/react-router'
import { MotionConfig } from 'motion/react'
import { NewProjectProvider } from './new-project/new-project-provider'
import { Sidebar } from './sidebar/sidebar'

export function Shell() {
  return (
    // `user`: with `prefers-reduced-motion`, Motion keeps fades and drops
    // every movement.
    <MotionConfig reducedMotion="user">
      <NewProjectProvider>
        <div className="flex h-dvh overflow-hidden">
          <Sidebar />
          <main className="flex min-w-0 flex-1 flex-col">
            <Outlet />
          </main>
        </div>
      </NewProjectProvider>
    </MotionConfig>
  )
}
