// @vitest-environment happy-dom
//
// The composer's `/` menu (§21.4) over a faked daemon: the harness's list
// arrives as a `commands` frame; the menu opens, filters, picks and closes.

import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { runningOperation } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

const COMMANDS = [
  { name: 'compact', description: 'Clear history', hint: null },
  { name: 'superpowers:brainstorming', description: 'Explore intent', hint: '[topic]' },
]

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  vi.restoreAllMocks()
})

/** A `commands` frame, and the query cache's notification that follows it
 * (the observers are told on the next tick, not inside the frame's act). */
async function deliver(a: TestApp, commands: readonly unknown[]) {
  act(() => a.pushFrame('commands', { thread_id: 't1', commands }))
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0))
  })
}

/** The conversation at t1, its stream caught up, the list delivered and the
 * session ready (Send acts), or a turn running (Enter would queue). */
async function open(running = false) {
  const daemon = {
    ...answers(running ? { operations: [runningOperation()] } : {}),
    'GET /api/threads/t1/queue': [],
    'POST /api/threads/t1/queue': () => new Response('refused', { status: 500 }),
  }
  const a = (app = await startApp('/projects/p1/threads/t1', daemon))
  await until(() => a.sources.length > 0 && a.container.querySelector('textarea') !== null)
  act(() => a.pushFrame('caught-up', { seq: 0 }))
  await deliver(a, COMMANDS)
  await until(() => a.button(running ? 'Stop' : 'fake-large') !== undefined)
  const box = a.container.querySelector('textarea')
  if (box === null) throw new Error('no message box')
  return { a, box }
}

const press = (box: HTMLTextAreaElement, key: string) =>
  act(() => {
    box.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }))
  })
const menu = () => document.querySelector('[role="listbox"]')
const names = () =>
  [...document.querySelectorAll('[role="option"]')].map((o) => o.getAttribute('data-name'))
const sentOrQueued = (a: TestApp) =>
  a.calls.filter((c) => c === 'POST /api/threads/t1/turns' || c === 'POST /api/threads/t1/queue')

it('opens on / and filters as the person types', async () => {
  const { box } = await open()
  typeInto(box, '/')
  expect(names()).toEqual(['compact', 'superpowers:brainstorming'])
  typeInto(box, '/br')
  expect(names()).toEqual(['superpowers:brainstorming'])
  typeInto(box, 'hi /br')
  expect(menu()).toBeNull()
})

it('Enter picks without sending, writes /name and shows the hint', async () => {
  const { a, box } = await open()
  typeInto(box, '/br')
  press(box, 'Enter')
  expect(box.value).toBe('/superpowers:brainstorming ')
  expect(a.text()).toContain('[topic]')
  expect(menu()).toBeNull()
  expect(sentOrQueued(a)).toEqual([])
})

it('the hint goes once the person types', async () => {
  const { a, box } = await open()
  typeInto(box, '/br')
  press(box, 'Enter')
  typeInto(box, '/superpowers:brainstorming x')
  expect(a.text()).not.toContain('[topic]')
})

it('arrows move, Tab picks, Escape closes and keeps the text', async () => {
  const { box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  press(box, 'Tab')
  expect(box.value).toBe('/superpowers:brainstorming ')
  typeInto(box, '/co')
  press(box, 'Escape')
  expect(menu()).toBeNull()
  expect(box.value).toBe('/co')
  typeInto(box, '/com')
  expect(names()).toEqual(['compact'])
})

it('a click picks', async () => {
  const { box } = await open()
  typeInto(box, '/')
  const option = document.querySelector('[data-name="compact"]')
  act(() => option?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })))
  expect(box.value).toBe('/compact ')
})

it('with no match the menu closes and Enter sends', async () => {
  const { a, box } = await open()
  typeInto(box, '/zzz')
  expect(menu()).toBeNull()
  press(box, 'Enter')
  await until(() => a.calls.includes('POST /api/threads/t1/turns'))
})

it('an empty list opens nothing', async () => {
  const { a, box } = await open()
  await deliver(a, [])
  typeInto(box, '/')
  expect(menu()).toBeNull()
})

it('Enter picks while a turn runs; it does not queue', async () => {
  const { a, box } = await open(true)
  typeInto(box, '/co')
  press(box, 'Enter')
  expect(box.value).toBe('/compact ')
  expect(sentOrQueued(a)).toEqual([])
})

it('a newline closes the menu', async () => {
  const { box } = await open()
  typeInto(box, '/co\nmore')
  expect(menu()).toBeNull()
})

it('a shorter new list clamps the highlight', async () => {
  const { a, box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  await deliver(a, [COMMANDS[0]])
  expect(names()).toEqual(['compact'])
  press(box, 'Enter')
  expect(box.value).toBe('/compact ')
})

it('the highlighted entry is scrolled into view', async () => {
  const scroll = vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(() => {})
  const { box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  expect(scroll).toHaveBeenCalled()
})

it('/model with a name sets the model picker and sends nothing', async () => {
  const { a, box } = await open()
  typeInto(box, '/model small')
  press(box, 'Enter')
  await until(() => a.calls.includes('PUT /api/threads/t1/session/model'))
  expect(a.bodies.at(-1)).toEqual({ model: 'fake-small' })
  expect(box.value).toBe('')
  expect(sentOrQueued(a)).toEqual([])
})

it('/model with a name the session does not offer says so', async () => {
  const { a, box } = await open()
  typeInto(box, '/model opus')
  press(box, 'Enter')
  expect(a.text()).toContain('This session offers no model named “opus”.')
  expect(box.value).toBe('/model opus')
  expect(sentOrQueued(a)).toEqual([])
})

it('picking model from the menu opens the model picker', async () => {
  const { a, box } = await open()
  await deliver(a, [{ name: 'model', description: 'Set the AI model', hint: '<model>' }])
  typeInto(box, '/mod')
  press(box, 'Enter')
  await until(() => document.querySelector('[role="menu"]') !== null)
  expect(document.querySelector('[role="menu"]')?.textContent).toContain('fake-small')
  expect(box.value).toBe('')
})
