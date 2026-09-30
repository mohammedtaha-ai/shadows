// @vitest-environment happy-dom
//
// Project settings over a faked daemon (§13.7, §13.8, §13.11): the Planner
// instructions saved as a command, and the external agents' grants — Connect
// with its one-time command, a replayed Connect, and Revoke.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { grantsQuery } from '@/api/queries'
import { grantFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

const PAGE = '/projects/p1/settings'
const NOTICE =
  "Claude Code stores this token in plain text in ~/.claude.json. Revoking it here stops Shadows accepting it; it does not remove it from Claude's settings."
const REPLAYED =
  'This connection was already created; its command is no longer shown. Revoke it and connect again if you need it.'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

const textarea = () => document.querySelector('textarea')
const codeBlocks = () => [...document.querySelectorAll('pre')]

function click(app: TestApp, name: string) {
  const button = app.button(name)
  if (button === undefined) throw new Error(`no button reads "${name}"`)
  act(() => button.click())
}

describe('project settings', () => {
  it('saving instructions sends the body with a command id', async () => {
    const app = (open = await startApp(
      PAGE,
      answers({
        instructions: { body: 'Plan in small steps.', number: 1, created_at: '2026-09-24T08:00:00Z' },
      }),
    ))
    await until(() => textarea()?.value === 'Plan in small steps.')
    expect(textarea()?.getAttribute('dir')).toBe('auto')
    expect(app.text()).toContain('Last changed')

    typeInto(textarea()!, 'اكتب الخطة بخطوات صغيرة.')
    click(app, 'Save')
    await until(() => app.calls.includes('PUT /api/projects/p1/planner-instructions'))
    const sent = app.bodies.at(-1) as { body: string; command_id: string }
    expect(sent.body).toBe('اكتب الخطة بخطوات صغيرة.')
    expect(sent.command_id).toMatch(/.+/)
    await until(() => app.text().includes('version 2'))
  })

  it('connect shows the command once with the notice', async () => {
    const app = (open = await startApp(PAGE, answers()))
    await until(() => app.button('Connect') !== undefined)
    click(app, 'Connect')
    await until(() => codeBlocks().length === 1)
    expect(app.bodies.at(-1)).toMatchObject({ command_id: expect.any(String) })
    expect(codeBlocks()[0]?.textContent).toContain('Authorization: Bearer tok-9')
    expect(app.button('Copy')).toBeDefined()
    expect(app.text()).toContain(NOTICE)
    // The new grant is listed, live.
    await until(() => app.buttons('Revoke').length === 1)

    // Leaving the page and coming back does not show the command again.
    const conversation = [...document.querySelectorAll('a')].find(
      (a) => a.textContent?.trim() === 'Conversation 1',
    )
    act(() => conversation?.click())
    await until(() => app.path() === '/projects/p1/threads/t1')
    const settings = [...document.querySelectorAll('a')].find(
      (a) => a.textContent?.trim() === 'Project settings',
    )
    act(() => settings?.click())
    await until(() => app.path() === PAGE && app.buttons('Revoke').length === 1)
    expect(codeBlocks()).toHaveLength(0)
    expect(app.text()).not.toContain('tok-9')
  })

  it('a replayed connect without a token explains why', async () => {
    const app = (open = await startApp(
      PAGE,
      answers({ issue: { grant: grantFixture('g9'), token: null, command: null } }),
    ))
    await until(() => app.button('Connect') !== undefined)
    click(app, 'Connect')
    await until(() => app.text().includes(REPLAYED))
    expect(codeBlocks()).toHaveLength(0)
    expect(app.text()).not.toContain(NOTICE)
  })

  it('revoke removes the grant from the list', async () => {
    const table = answers({ grants: [grantFixture('g1')] })
    const urls: string[] = []
    const revoke = table['DELETE /api/mcp-grants/g1'] as (r: Request) => Response
    table['DELETE /api/mcp-grants/g1'] = (r: Request) => {
      urls.push(r.url)
      return revoke(r)
    }
    const app = (open = await startApp(PAGE, table))
    await until(() => app.buttons('Revoke').length === 1)
    click(app, 'Revoke')
    await until(() => app.text().includes('Revoked'))
    expect(new URL(urls[0]!).searchParams.get('command_id')).toMatch(/.+/)
    expect(app.buttons('Revoke')).toHaveLength(0)
  })

  it('a revoked grant shows as revoked, with no Revoke button', async () => {
    const app = (open = await startApp(
      PAGE,
      answers({
        grants: [
          grantFixture('g2'),
          grantFixture('g1', { revoked_at: '2026-09-24T12:00:00Z' }),
          // A Planner's own grant is not the person's to manage (§13.7).
          grantFixture('g0', { kind: 'thread', thread_id: 't1' }),
        ],
      }),
    ))
    await until(() => app.text().includes('Revoked'))
    expect(app.buttons('Revoke')).toHaveLength(1)
    expect(document.querySelectorAll('[data-grant]')).toHaveLength(2)
    // A screen reader hears which connection the button ends.
    const name = app.buttons('Revoke')[0]?.getAttribute('aria-label') ?? ''
    expect(name).toMatch(/^Revoke the connection made .+, g2$/)
  })

  it('a newer version fetched in the background keeps unsaved edits', async () => {
    const table = answers({
      instructions: { body: 'Plan in small steps.', number: 1, created_at: '2026-09-24T08:00:00Z' },
    })
    const app = (open = await startApp(PAGE, table))
    await until(() => textarea()?.value === 'Plan in small steps.')
    typeInto(textarea()!, 'Plan in small steps, and ask first.')

    // Another tab saved version 2; a refetch (on focus, say) brings it here.
    table['GET /api/projects/p1/planner-instructions'] = {
      body: 'Saved elsewhere.',
      number: 2,
      created_at: '2026-09-25T08:00:00Z',
    }
    await act(() => app.queryClient.invalidateQueries())
    await until(() => app.text().includes('version 2'))
    expect(textarea()?.value).toBe('Plan in small steps, and ask first.')
  })

  it("another project's settings do not show this one's command or draft", async () => {
    const table = answers()
    table['GET /api/projects/p2/planner-instructions'] = null
    table['GET /api/projects/p2/mcp-grants'] = []
    const app = (open = await startApp(PAGE, table))
    await until(() => app.button('Connect') !== undefined && textarea() !== null)
    typeInto(textarea()!, 'Only for p1.')
    click(app, 'Connect')
    await until(() => codeBlocks().length === 1)

    const { router } = await import('@/router')
    await act(() =>
      router.navigate({ to: '/projects/$projectId/settings', params: { projectId: 'p2' } }),
    )
    await until(() => app.calls.includes('GET /api/projects/p2/mcp-grants'))
    await until(() => textarea() !== null)
    expect(codeBlocks()).toHaveLength(0)
    expect(textarea()?.value).toBe('')
  })

  it('the grant list is polled every 10 s while the page is open', async () => {
    const app = (open = await startApp(PAGE, answers()))
    await until(() => app.calls.includes('GET /api/projects/p1/mcp-grants'))
    const query = app.queryClient.getQueryCache().find({ queryKey: grantsQuery('p1').queryKey })
    expect(query?.options).toMatchObject({ refetchInterval: 10_000 })
  })
})
