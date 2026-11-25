// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { projectExtensionsQuery, type AgentExtensions } from '@/api/extensions'
import { projectsQuery, type Project } from '@/api/projects'
import { projectFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type Answer, type TestApp } from '@/test/test-app'

const PAGE = '/projects/p1/settings'
const skill = { id: '22222222-2222-4222-8222-222222222222', name: 'review', description: 'Review changes.' }
const mcp = { id: 'm1', name: 'research', description: 'Research tools', transport: { type: 'stdio', command: 'x', args: [], env: [] } }
const plugin = { id: '11111111-1111-4111-8111-111111111111', name: 'git', description: 'Git tools.' }
const view = (skillOn = false, mcpOn = false): AgentExtensions =>
  ({ skills: [{ ...skill, enabled: skillOn }], mcp: [{ ...mcp, enabled: mcpOn }], plugins: [{ ...plugin, enabled: true }] }) as AgentExtensions
const GET = 'GET /api/projects/p1/planner-extensions'
const SKILL_PUT = `PUT /api/projects/p1/planner-extensions/skill/${skill.id}`
const CLAUDE_PUT = 'PUT /api/projects/p1/planner-claude-md'
const ID = { command_id: expect.any(String) }

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const switchOf = (name: string) => document.querySelector<HTMLInputElement>(`input[role="switch"][aria-label="${name}"]`)!
const CLAUDE = "Read the project's CLAUDE.md"
const count = (a: TestApp, key: string) => a.calls.filter(call => call === key).length
async function open(overrides: Record<string, Answer> = {}, ready = 'review') {
  const a = app = await startApp(PAGE, { ...answers(), [GET]: () => Response.json(view()), ...overrides })
  await until(() => switchOf(ready) !== null)
  return a
}
const failure = (status: number, code: string, message: string) => Response.json({ code, message }, { status })

describe('the Planner switches', () => {
  it("lists the library's skills and MCP servers, never a plugin, with no tool rules", async () => {
    const a = await open({ [GET]: () => Response.json(view(false, true)) })
    expect(switchOf('research').checked).toBe(true)
    expect(switchOf(CLAUDE)).not.toBeNull()
    expect(switchOf('git')).toBeNull()
    expect(a.text()).not.toContain('Plugins')
    expect(a.text()).toContain('These switches apply to the Planner on Claude Code only.')
    expect(a.calls.some(call => call.endsWith('/tools'))).toBe(false)
  })

  it('a switch saves at once with a command id', async () => {
    let resolve!: (response: Response) => void
    let on = false
    const a = await open({ [GET]: () => Response.json(view(on)),
      [SKILL_PUT]: () => new Promise<Response>(done => { resolve = done }) })
    act(() => switchOf('review').click()); await until(() => a.bodies.length === 1)
    expect(a.calls).toContain(SKILL_PUT)
    expect(a.bodies[0]).toEqual({ enabled: true, ...ID })
    expect(switchOf('review').checked).toBe(true); expect(switchOf('review').disabled).toBe(true)
    on = true; await act(async () => resolve(Response.json({ ...skill, enabled: true })))
    await until(() => switchOf('review').checked && !switchOf('review').disabled)
    expect(a.queryClient.getQueryData<AgentExtensions>(projectExtensionsQuery('p1').queryKey)?.skills[0].enabled).toBe(true)
  })

  it('a failed switch goes back, and its retry resends the same command id', async () => {
    let n = 0, on = false
    const a = await open({ [GET]: () => Response.json(view(on)),
      [SKILL_PUT]: async (request: Request) => {
        const input = await request.json()
        if (++n === 1) return failure(500, 'STORAGE_UNAVAILABLE', 'Toggle failed.')
        on = input.enabled; return Response.json({ ...skill, enabled: on })
      } })
    act(() => switchOf('review').click()); await until(() => a.text().includes('Toggle failed.') && !switchOf('review').disabled)
    expect(switchOf('review').checked).toBe(false)
    act(() => switchOf('review').click()); await until(() => n === 2 && !switchOf('review').disabled)
    expect(a.bodies[1]).toEqual(a.bodies[0])
    act(() => switchOf('review').click()); await until(() => n === 3 && !switchOf('review').disabled)
    expect((a.bodies[2] as { command_id: string }).command_id).not.toBe((a.bodies[0] as { command_id: string }).command_id)
  })

  it('a switch whose item is gone refetches the list', async () => {
    let gone = false
    const a = await open({ [GET]: () => Response.json(gone ? { skills: [], mcp: [{ ...mcp, enabled: false }], plugins: [] } : view()),
      [SKILL_PUT]: () => { gone = true; return failure(404, 'EXTENSION_NOT_FOUND', 'extension not found') } })
    act(() => switchOf('review').click())
    await until(() => count(a, GET) === 2 && switchOf('review') === null)
    expect(switchOf('research')).not.toBeNull()
  })

  it('a conflict is shown and not retried', async () => {
    const message = 'command conflict: the same command id was reused with a different request'
    const a = await open({ [SKILL_PUT]: () => failure(409, 'COMMAND_CONFLICT', message) })
    act(() => switchOf('review').click()); await until(() => a.text().includes(message) && !switchOf('review').disabled)
    expect(switchOf('review').checked).toBe(false)
    expect(count(a, SKILL_PUT)).toBe(1)
  })

  it('the CLAUDE.md switch saves the project', async () => {
    const a = await open({ [CLAUDE_PUT]: () => Response.json({ ...projectFixture, planner_reads_claude_md: true }) })
    act(() => switchOf(CLAUDE).click()); await until(() => a.bodies.length === 1)
    expect(a.bodies[0]).toEqual({ enabled: true, ...ID })
    await until(() => switchOf(CLAUDE).checked && !switchOf(CLAUDE).disabled)
    expect(a.queryClient.getQueryData<Project[]>(projectsQuery.queryKey)?.[0].planner_reads_claude_md).toBe(true)
  })

  it('the CLAUDE.md switch goes back when the project is gone', async () => {
    const a = await open({ [CLAUDE_PUT]: () => failure(404, 'INVALID_COMMAND', 'not found: project') })
    act(() => switchOf(CLAUDE).click()); await until(() => a.text().includes('not found: project') && !switchOf(CLAUDE).disabled)
    expect(switchOf(CLAUDE).checked).toBe(false)
    expect(count(a, GET)).toBe(1)
  })

  it('an empty library says so, links to the library, and keeps the CLAUDE.md switch', async () => {
    const a = app = await startApp(PAGE, { ...answers(), [GET]: { skills: [], mcp: [], plugins: [] },
      [CLAUDE_PUT]: () => Response.json({ ...projectFixture, planner_reads_claude_md: true }) })
    await until(() => a.text().includes('No skills or MCP servers in the library.'))
    act(() => switchOf(CLAUDE).click()); await until(() => switchOf(CLAUDE).checked && !switchOf(CLAUDE).disabled)
    act(() => a.button('Add to library')!.click())
    await until(() => document.querySelector('[data-slot="dialog-title"]')?.textContent === 'Plugins & Skills')
    expect(a.path()).toBe(PAGE)
  })

  it("a failed load shows Core's message with Retry, and the CLAUDE.md switch still works", async () => {
    const a = app = await startApp(PAGE, { ...answers(), [GET]: () => failure(404, 'EXTENSION_NOT_FOUND', 'extension not found') })
    await until(() => a.text().includes('extension not found'))
    expect(a.text()).not.toContain('No skills or MCP servers in the library.')
    expect(switchOf(CLAUDE)).not.toBeNull()
    act(() => a.button('Retry')!.click()); await until(() => count(a, GET) === 2)
  })

  it('a late answer stays with its project', async () => {
    let resolve!: (response: Response) => void
    const p2 = { ...projectFixture, id: 'p2', slug: 'p2', name: 'Second' }
    const a = await open({ 'GET /api/projects': () => Response.json([projectFixture, p2]),
      'GET /api/projects/p2/planner-extensions': () => Response.json(view()),
      'GET /api/projects/p2/planner-instructions': { body: '', number: 0, created_at: 'now' },
      'GET /api/projects/p2/code/links': [], 'GET /api/projects/p2/code/servers': [],
      [SKILL_PUT]: () => new Promise<Response>(done => { resolve = done }) })
    act(() => switchOf('review').click()); await until(() => a.bodies.length === 1)
    act(() => { window.history.pushState(null, '', '/projects/p2/settings'); window.dispatchEvent(new PopStateEvent('popstate')) })
    await until(() => a.calls.includes('GET /api/projects/p2/planner-extensions'))
    const before = a.queryClient.getQueryData(projectExtensionsQuery('p2').queryKey)
    await act(async () => resolve(Response.json({ ...skill, enabled: true })))
    expect(a.queryClient.getQueryData(projectExtensionsQuery('p2').queryKey)).toEqual(before)
    await until(() => switchOf('review') !== null)
    expect(switchOf('review').checked).toBe(false)
  })
})
