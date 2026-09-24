// @vitest-environment happy-dom
//
// The project page's allowed modes (spec §12.5): a per-harness checklist that
// changes the project's set as one command, and says when none is left.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
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
    await until(() => app.text().includes('No mode left: turns cannot start'))
    expect(app.bodies.at(-1)).toMatchObject({ allowed_modes: { 'claude-code': [] } })
    const [first, second] = app.bodies.slice(-2) as { command_id: string }[]
    expect(second.command_id).not.toBe(first.command_id)
  })

  it('a harness whose modes are not decided says so instead of a checklist', async () => {
    const app = (open = await startApp('/projects/p1', answers()))
    await until(() => app.checkbox('Auto') !== undefined)
    expect(app.text()).toContain('Codex')
    expect(app.text()).toContain('Modes are decided when Codex is enabled')
  })
})
