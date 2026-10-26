// One job: the two-pane frame every screen sits in.

import { Outlet, useParams } from '@tanstack/react-router'
import { MotionConfig } from 'motion/react'
import { useProjectEvents } from '@/stream'
import { NewProjectProvider } from '@/app/new-project'
import { Sidebar } from '@/app/sidebar'
import { SettingsDialog } from '@/app/settings'
import { PersonTheme } from '@/app/settings'

export function Shell() {
  const { projectId } = useParams({ strict: false })
  useProjectEvents(projectId)
  return (
    // `user`: with `prefers-reduced-motion`, Motion keeps fades and drops
    // every movement.
    <MotionConfig reducedMotion="user">
      <NewProjectProvider>
        <PersonTheme />
        <div className="flex h-dvh overflow-hidden">
          <Sidebar />
          <main className="flex min-w-0 flex-1 flex-col">
            <Outlet />
          </main>
        </div>
        <SettingsDialog />
      </NewProjectProvider>
    </MotionConfig>
  )
}
