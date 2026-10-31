// One job: the two-pane frame every screen sits in.

import { Outlet, useParams } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
import { AnimatePresence, MotionConfig } from 'motion/react'
import { useProjectEvents } from '@/stream'
import { NewProjectProvider } from '@/app/new-project'
import { Sidebar, SidebarToggle, useSidebarState } from '@/app/sidebar'
import { SettingsDialog } from '@/app/settings'
import { PersonTheme } from '@/app/settings'

export function Shell() {
  const { projectId } = useParams({ strict: false })
  useProjectEvents(projectId)
  const sidebar = useSidebarState()
  const previous = useRef(sidebar.shown)
  useEffect(() => {
    if (!previous.current && sidebar.shown) document.querySelector<HTMLButtonElement>('#app-sidebar [aria-label="Hide sidebar"]')?.focus()
    previous.current = sidebar.shown
  }, [sidebar.shown])
  return (
    // `user`: with `prefers-reduced-motion`, Motion keeps fades and drops
    // every movement.
    <MotionConfig reducedMotion="user">
      <NewProjectProvider>
        <PersonTheme />
        <div className="flex h-dvh overflow-hidden">
          <AnimatePresence initial={false}>{sidebar.shown && <Sidebar key="sidebar" narrow={sidebar.narrow} onToggle={sidebar.toggle} onHide={sidebar.hide} />}</AnimatePresence>
          <main className={`relative flex min-w-0 flex-1 flex-col ${!sidebar.shown ? '[&>div>div>header]:pl-16' : ''}`}>
            {!sidebar.shown && <SidebarToggle shown={false} onToggle={sidebar.toggle} className="absolute top-2.5 left-2.5 z-20" />}
            <Outlet />
          </main>
        </div>
        <SettingsDialog />
      </NewProjectProvider>
    </MotionConfig>
  )
}
