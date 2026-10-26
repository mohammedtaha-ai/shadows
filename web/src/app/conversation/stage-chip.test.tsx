// @vitest-environment happy-dom
//
// The stage line is the Planner's lifecycle: shown in a normal project, never
// in the assistant project.

import { afterEach, describe, expect, it } from 'vitest'
import { projectFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('the stage chip', () => {
  it('stage_chip_hidden_in_the_assistant_project', async () => {
    const normal = (open = await startApp('/projects/p1/new', answers()))
    await until(() => normal.text().includes('Stage: idea'))
    normal.unmount()

    const table = answers()
    table['GET /api/projects'] = [{ ...projectFixture, assistant: true }]
    const aylul = (open = await startApp('/projects/p1/new', table))
    await until(() => aylul.button('Send') !== undefined || aylul.text().includes('first message'))
    await new Promise((r) => setTimeout(r, 200))
    expect(aylul.text()).not.toContain('Stage:')
  })
})
