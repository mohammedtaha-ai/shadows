// @vitest-environment happy-dom
//
// The project page's allowed modes (spec §12.5): a per-harness checklist that
// changes the project's set as one command, and says when none is left.

import { notifyManager } from '@tanstack/react-query'
import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { Project } from '@/api/client'
import { projectsQuery } from '@/api/queries'
import { projectWithModes } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from './test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('allowed modes', () => {
  it('the project page edits allowed modes', async () => {
    const app = (open = await startApp(
      '/projects/p1',
      answers({ project: projectWithModes({ 'claude-code': ['acceptEdits', 'auto'] }) }),
    ))
    await until(() => app.checkbox('Auto') !== undefined)
    act(() => app.checkbox('Auto')?.click())
    await until(() => app.calls.includes('PATCH /api/projects/p1'))
    expect(app.bodies.at(-1)).toMatchObject({ allowed_modes: { 'claude-code': ['acceptEdits'] } })
    act(() => app.checkbox('Accept edits')?.click())
    // Shown at once, while the second change waits behind the first.
    await until(() => app.text().includes('No mode left: turns cannot start'))
    await until(() => app.bodies.length === 2)
    expect(app.bodies.at(-1)).toMatchObject({ allowed_modes: { 'claude-code': [] } })
    const [first, second] = app.bodies.slice(-2) as { command_id: string }[]
    expect(second.command_id).not.toBe(first.command_id)
  })

  it('a click after a save uses that save while the project query has not rendered', async () => {
    const app = (open = await startApp(
      '/projects/p1',
      answers({ project: projectWithModes({ 'claude-code': ['acceptEdits', 'auto'] }) }),
    ))
    await until(() => app.checkbox('Auto') !== undefined)
    const scheduled: (() => void)[] = []
    notifyManager.setScheduler((callback) => scheduled.push(callback))
    try {
      act(() => app.checkbox('Auto')?.click())
      // The cache has the PATCH answer, but its subscriber notifications are
      // held, so the parent still supplies its previous project prop.
      await until(() => {
        const projects = app.queryClient.getQueryData<Project[]>(projectsQuery.queryKey)
        return projects?.[0]?.allowed_modes['claude-code']?.includes('auto') === false
      })
      expect(app.checkbox('Auto')?.checked).toBe(false)
      act(() => app.checkbox('Accept edits')?.click())
      await until(() => app.bodies.length === 2)
      expect(app.bodies[1]).toMatchObject({ allowed_modes: { 'claude-code': [] } })
    } finally {
      notifyManager.setScheduler((callback) => setTimeout(callback, 0))
      await act(async () => {
        for (const callback of scheduled) callback()
      })
    }
  })

  it('a harness whose modes are not decided says so instead of a checklist', async () => {
    const app = (open = await startApp('/projects/p1', answers()))
    await until(() => app.checkbox('Auto') !== undefined)
    expect(app.text()).toContain('Codex')
    expect(app.text()).toContain('Modes are decided when Codex is enabled')
  })
})
