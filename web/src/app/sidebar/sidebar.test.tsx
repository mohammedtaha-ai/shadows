// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '@/test/test-app'
let app: TestApp | null = null
const sidebar = () => document.querySelector('#app-sidebar')
const button = (s: string) => document.querySelector<HTMLButtonElement>(`button[aria-label="${s}"]`)
const click = (s: string) => act(() => button(s)?.click())
const input = () => document.querySelector<HTMLInputElement>('input[aria-label="Filter sidebar"]')
const media = (narrow: boolean) => vi.stubGlobal('matchMedia', vi.fn().mockReturnValue({ matches: narrow, addEventListener: vi.fn(), removeEventListener: vi.fn() }))
afterEach(() => { app?.unmount(); app = null; vi.unstubAllGlobals(); vi.restoreAllMocks(); localStorage.clear() })
it('the sidebar shows the description under the agent name', async () => {
  app = await startApp('/aylul', { ...answers(),
    'GET /api/aylul/providers': [{ slug: 'fixture', models: [] }],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [{ slug: 'work', name: 'Work', description: 'Builds useful things' }] },
  })
  await until(() => sidebar()?.textContent?.includes('Builds useful things') === true)
  const row = [...sidebar()!.querySelectorAll('button')].find(button => button.textContent?.includes('Work'))!
  expect(row.textContent).toContain('WorkBuilds useful things')
})
it('the agent filter narrows by name and by description, and says when nothing matches', async () => {
  app = await startApp('/aylul', { ...answers(),
    'GET /api/aylul/providers': [{ slug: 'fixture', models: [] }],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [
      { slug: 'work', name: 'Work', description: 'Builds useful things' },
      { slug: 'play', name: 'Play', description: 'Tells jokes' },
    ] },
  })
  await until(() => sidebar()?.textContent?.includes('Tells jokes') === true)
  act(() => button('Search sidebar')?.click())
  await until(() => input() !== null)
  const type = (text: string) => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!
    act(() => { set.call(input(), text); input()!.dispatchEvent(new Event('input', { bubbles: true })) })
  }
  type('wor')
  expect(sidebar()!.textContent).not.toContain('Tells jokes')
  expect(sidebar()!.textContent).toContain('Builds useful things')
  type('jokes')
  expect(sidebar()!.textContent).toContain('Tells jokes')
  expect(sidebar()!.textContent).not.toContain('Builds useful things')
  type('zzz')
  expect(sidebar()!.textContent).toContain('No matching agents.')
})
it("the sidebar's button hides it, the main pane's button and Ctrl+B show it again", async () => {
 media(false); app = await startApp('/projects/p1/threads/t1', answers()); await until(() => sidebar() !== null)
 expect(button('Hide sidebar')?.getAttribute('aria-expanded')).toBe('true'); click('Hide sidebar'); await until(() => sidebar() === null)
 expect(button('Show sidebar')?.getAttribute('aria-expanded')).toBe('false'); click('Show sidebar'); await until(() => sidebar() !== null); await until(() => document.activeElement === button('Hide sidebar'))
 click('Hide sidebar'); await until(() => sidebar() === null); act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }))); await until(() => sidebar() !== null)
})
it('a hidden sidebar stays hidden after a reload', async () => {
 media(false); app = await startApp('/projects/p1/threads/t1', answers()); await until(() => sidebar() !== null); click('Hide sidebar'); await until(() => sidebar() === null)
 expect(localStorage.getItem('shadows.sidebar.collapsed')).toBe('true'); app.unmount(); app = await startApp('/projects/p1/threads/t1', answers()); expect(sidebar()).toBeNull()
})
it('without storage the choice lasts until reload', async () => {
 media(false); vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw Error('blocked') }); vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw Error('blocked') })
 app = await startApp('/projects/p1/threads/t1', answers()); await until(() => sidebar() !== null); click('Hide sidebar'); await until(() => sidebar() === null); click('Show sidebar'); await until(() => sidebar() !== null)
})
it('below 768 px it starts hidden, shows as an overlay, and closes on a chosen conversation, on Escape and on the backdrop', async () => {
 localStorage.setItem('shadows.sidebar.collapsed', 'false'); const writes = vi.spyOn(Storage.prototype, 'setItem'); media(true); app = await startApp('/projects/p1/new', answers()); expect(sidebar()).toBeNull()
 click('Show sidebar'); await until(() => sidebar() !== null); expect(sidebar()?.className).toContain('fixed'); expect(localStorage.getItem('shadows.sidebar.collapsed')).toBe('false'); expect(writes.mock.calls.filter(([key]) => key === 'shadows.sidebar.collapsed')).toHaveLength(0)
 act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))); await until(() => sidebar() === null); await until(() => document.activeElement === button('Show sidebar'))
 click('Show sidebar'); await until(() => sidebar() !== null); click('Close sidebar'); await until(() => sidebar() === null)
 click('Show sidebar'); await until(() => [...document.querySelectorAll('nav a')].some(a => a.textContent?.includes('Conversation 1'))); act(() => [...document.querySelectorAll<HTMLAnchorElement>('nav a')].find(a => a.textContent?.includes('Conversation 1'))?.click()); await until(() => sidebar() === null)
})
