// @vitest-environment happy-dom
//
// AYLUL's conversation: model and effort come from the session's choices, and
// the CLI menu offers `aylul` in the assistant project only, and only it there.

import { afterEach, describe, expect, it } from 'vitest'
import { claudeHarness, projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, menuItem, startApp, until } from '../test-app'
import { act } from 'react'

const aylulHarness = { ...claudeHarness, kind: 'aylul', label: 'AYLUL' }
const assistant = { ...projectFixture, assistant: true, allowed_modes: { aylul: ['acceptEdits', 'auto'] } }
const aylulThread = { ...threadFixture, harness: 'aylul' }

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('AYLUL in the web client', () => {
  it('aylul_composer_offers_models_and_efforts_from_choices', async () => {
    const calls: string[] = []
    const table = answers()
    table['GET /api/harnesses'] = [claudeHarness, aylulHarness]
    table['GET /api/projects'] = [assistant]
    table['GET /api/threads/t1'] = aylulThread
    table['GET /api/projects/p1/threads'] = [aylulThread]
    const app = (open = await startApp('/projects/p1/threads/t1', table))
    await until(() => app.button('fake-large') !== undefined && app.button('high') !== undefined)
    act(() => app.button('fake-large')?.click())
    await until(() => menuItem('fake-small') !== undefined)
    act(() => menuItem('fake-small')?.click())
    await until(() => app.calls.includes('PUT /api/threads/t1/session/model'))
    calls.push(...app.calls)
    expect(calls).toContain('PUT /api/threads/t1/session/model')
  })

  it('the CLI menu offers only aylul in the assistant project, and never in another', async () => {
    const table = answers()
    table['GET /api/harnesses'] = [claudeHarness, aylulHarness]

    table['GET /api/projects'] = [assistant]
    const a = (open = await startApp('/projects/p1/new', table))
    await until(() => a.button('AYLUL') !== undefined)
    act(() => a.button('AYLUL')?.click())
    await until(() => menuItem('AYLUL') !== undefined)
    expect(menuItem('Claude Code')).toBeUndefined()
    a.unmount()

    table['GET /api/projects'] = [projectFixture]
    const b = (open = await startApp('/projects/p1/new', table))
    await until(() => b.button('Claude Code') !== undefined)
    act(() => b.button('Claude Code')?.click())
    await until(() => menuItem('Claude Code') !== undefined)
    expect(menuItem('AYLUL')).toBeUndefined()
  })
})
