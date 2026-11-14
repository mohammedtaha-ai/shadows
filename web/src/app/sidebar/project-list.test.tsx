// @vitest-environment happy-dom
//
// The sidebar's projects as a tree: each folds on its own, several stay open,
// the open set outlives a reload, and a row's `+` opens a new conversation's
// draft, which makes nothing on the daemon.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { Answer } from '@/test/test-app'
import { threadsQuery } from '@/api/threads'
import { projectsQuery } from '@/api/projects'
import { projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

const second = { ...projectFixture, id: 'p2', slug: 'second', name: 'Second', directory: null }
const secondThread = { ...threadFixture, id: 't2', project_id: 'p2', title: 'Other talk' }

function twoProjects(): Record<string, Answer> {
  return {
    ...answers(),
    'GET /api/projects': [projectFixture, second],
    'GET /api/projects/p2/threads': [secondThread],
    'GET /api/projects/p2/workflows': [],
  }
}

/** The fold button of the project named `name`. */
const toggleOf = (name: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button[aria-expanded]')].find((b) =>
    b.textContent?.includes(name),
  )
const expanded = (name: string) => toggleOf(name)?.getAttribute('aria-expanded') === 'true'
const link = (text: string) =>
  [...document.querySelectorAll('nav a')].find((a) => a.textContent?.trim() === text)

/** Types into the sidebar's filter the way a person does. */
function typeFilter(text: string) {
  const input = document.querySelector<HTMLInputElement>('input[aria-label="Filter sidebar"]')!
  const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!
  act(() => { set.call(input, text); input.dispatchEvent(new Event('input', { bubbles: true })) })
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  window.localStorage.clear()
})

describe('the project tree', () => {
  it('shows project skeletons until the first answer, then replaces them with projects', async () => {
    let answer!: (response: Response) => void
    const pending = new Promise<Response>(resolve => { answer = resolve })
    const a = app = await startApp('/', { ...answers(), 'GET /api/projects': () => pending })
    const loading = () => a.container.querySelector('[role="status"][aria-label="Loading projects…"]')
    try {
      await until(() => loading() !== null)
      expect(loading()?.querySelector('[data-slot="skeleton"]')).not.toBeNull()
      expect(a.text()).not.toContain('Create your first project')
    } finally {
      await act(async () => { answer(Response.json([projectFixture])) })
    }
    await until(() => a.container.querySelector('h2')?.textContent === 'Demo')
    expect(loading()).toBeNull()
  })

  it('keeps the project cards visible during a background refresh', async () => {
    let answer!: (response: Response) => void
    const pending = new Promise<Response>(resolve => { answer = resolve })
    let refreshing = false
    const a = app = await startApp('/', { ...answers(),
      'GET /api/projects': () => refreshing ? pending : Response.json([projectFixture]),
    })
    await until(() => a.container.querySelector('h2')?.textContent === 'Demo')
    refreshing = true
    act(() => { void a.queryClient.invalidateQueries({ queryKey: projectsQuery.queryKey, exact: true }) })
    try {
      await until(() => a.calls.filter(call => call === 'GET /api/projects').length === 2)
      expect(a.container.querySelector('h2')?.textContent).toBe('Demo')
      expect(a.container.querySelector('[aria-label="Loading projects…"]')).toBeNull()
    } finally {
      await act(async () => { answer(Response.json([])) })
    }
    await until(() => a.text().includes('Create your first project'))
  })

  it('replaces project skeletons with the daemon failure and retry control', async () => {
    const a = app = await startApp('/', { ...answers(), 'GET /api/projects': () =>
      Response.json({ code: 'unavailable', message: 'Daemon unavailable' }, { status: 503 }),
    })
    await until(() => a.button('Try again') !== undefined)
    expect(a.container.querySelector('[aria-label="Loading projects…"]')).toBeNull()
    expect(a.container.querySelector('[aria-label="Loading project list…"]')).toBeNull()
  })

  it('folds each project on its own, keeps the conversation, and remembers across a reload', async () => {
    const a = (app = await startApp('/projects/p1/threads/t1', twoProjects()))
    // The URL's project opens by itself; the other stays folded.
    await until(() => link('Conversation 1') !== undefined)
    expect(expanded('Demo')).toBe(true)
    expect(expanded('Second')).toBe(false)
    const demo = toggleOf('Demo')
    expect(document.getElementById(demo?.getAttribute('aria-controls') ?? '')).not.toBeNull()

    // Folding the project that holds the conversation stays on it.
    act(() => toggleOf('Demo')?.click())
    expect(link('Conversation 1')).toBeUndefined()
    expect(a.path()).toBe('/projects/p1/threads/t1')

    // Two open at once.
    act(() => toggleOf('Second')?.click())
    act(() => toggleOf('Demo')?.click())
    await until(() => link('Other talk') !== undefined && link('Conversation 1') !== undefined)
    a.unmount()

    // After a reload elsewhere, both are still open.
    app = await startApp('/', twoProjects())
    await until(() => link('Other talk') !== undefined)
    expect(expanded('Demo')).toBe(true)
    expect(expanded('Second')).toBe(true)
  })

  it('a row’s + opens a draft in that project and creates nothing', async () => {
    const a = (app = await startApp('/projects/p1/threads/t1', twoProjects()))
    // The folded project's + works without opening it.
    const plus = () =>
      document.querySelector<HTMLAnchorElement>('a[aria-label="New conversation in Second"]')
    await until(() => plus() !== null)
    act(() => plus()?.click())

    await until(() => a.path() === '/projects/p2/new')
    await until(() => a.text().includes('The conversation starts with your first message'))
    expect(a.calls.filter((c) => c.endsWith('/threads') && c.startsWith('POST'))).toEqual([])
  })

  it('an open project’s conversations are polled, so a title the harness sends later shows', async () => {
    const a = (app = await startApp('/projects/p1/threads/t1', twoProjects()))
    await until(() => link('Conversation 1') !== undefined)
    const query = a.queryClient.getQueryCache().find({ queryKey: threadsQuery('p1').queryKey })
    expect(query?.options).toMatchObject({ refetchInterval: 10_000 })
  })

  it('the filter narrows projects by name and by directory, and says when nothing matches', async () => {
    app = await startApp('/', twoProjects())
    await until(() => toggleOf('Second') !== undefined)
    act(() => document.querySelector<HTMLButtonElement>('button[aria-label="Search sidebar"]')?.click())
    await until(() => document.querySelector('input[aria-label="Filter sidebar"]') !== null)
    typeFilter('sec')
    expect(toggleOf('Demo')).toBeUndefined()
    expect(toggleOf('Second')).toBeDefined()
    typeFilter('work')
    expect(toggleOf('Demo')).toBeDefined()
    expect(toggleOf('Second')).toBeUndefined()
    typeFilter('zzz')
    expect(document.querySelector('nav')?.textContent).toContain('No matching projects.')
  })

  it('Escape closes the search and clears the filter', async () => {
    app = await startApp('/', twoProjects())
    await until(() => toggleOf('Second') !== undefined)
    act(() => document.querySelector<HTMLButtonElement>('button[aria-label="Search sidebar"]')?.click())
    await until(() => document.querySelector('input[aria-label="Filter sidebar"]') !== null)
    typeFilter('sec')
    expect(toggleOf('Demo')).toBeUndefined()
    const input = document.querySelector('input[aria-label="Filter sidebar"]')!
    act(() => { input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })) })
    expect(document.querySelector('input[aria-label="Filter sidebar"]')).toBeNull()
    expect(toggleOf('Demo')).toBeDefined()
  })
})
