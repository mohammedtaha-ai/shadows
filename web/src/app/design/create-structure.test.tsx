// @vitest-environment happy-dom
import { designKeys } from '@/api/design'
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers as baseAnswers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const part = { id: 'backend', parent: null, ordinal: 0, revision: 1,
  content: { title: 'Backend', responsibility: 'Logic', design: '', kind: 'backend',
    folders: ['backend'], files: [{path:'backend/main.rs',content:'hello'}],
    serves: [], contract: {header:'Head',rules:[]} } }
function answers(stage = 'structure', missing: string[] = [], map = { revision: 1, count: 3 as number | null }) {
  return { ...baseAnswers({ map }),
    'GET /api/projects/p1/stage': () => Response.json({stage,missing,drift:[],
      structure_file_count:map.count,map_revision:map.revision}),
    'GET /api/projects/p1/design/parts': (request: Request) => Response.json({revision:1,
      items: new URL(request.url).searchParams.has('parent') ? [] : [part], next:null}),
  }
}

it('enables Create structure for a complete approved map', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', answers())
  await until(() => a.button('Create structure')?.disabled === false)
  expect(a.calls.filter(c => c.startsWith('POST'))).toHaveLength(0)
})

it('disables Create structure with the missing mandatory kind', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', answers('structure',['billing']))
  await until(() => a.button('Create structure') !== undefined)
  expect(a.button('Create structure')?.disabled).toBe(true)
  expect(a.text()).toContain('missing part: billing')
})

it('confirms the directory and count, writes, then preserves success after roadmap refresh', async () => {
  let written = false
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(),
    'GET /api/projects/p1/stage': () => Response.json({stage:written?'roadmap':'structure',
      missing:[],drift:[],structure_file_count:3,map_revision:1}),
    'POST /api/projects/p1/structure': () => {
      written = true
      return Response.json({map_version:2,paths:['SHADOWS.md','backend/contract.yaml','backend/main.rs'],
        completed_at:'2026-10-12T00:00:00Z',resumed:false})
    },
  })
  await until(() => a.button('Create structure')?.disabled === false)
  act(() => a.button('Create structure')?.click())
  await until(() => a.button('Confirm create structure')?.disabled === false)
  const dialog = a.container.querySelector('[role="dialog"]')!
  expect(dialog.textContent).toContain('C:\\work\\demo')
  expect(dialog.textContent).toContain('3 files')
  expect(a.calls.filter(c => c.startsWith('POST'))).toHaveLength(0)
  act(() => a.button('Confirm create structure')?.click())
  await until(() => a.text().includes('Structure written · 3 files'))
  expect(a.text()).toContain('backend/contract.yaml')
  expect(a.bodies.at(-1)).toMatchObject({command_id:expect.any(String),map_revision:1})
  await until(() => a.button('Create structure') === undefined)
  expect(a.queryClient.getQueryData(designKeys.stage('p1'))).toMatchObject({stage:'roadmap'})
})

it('shows the daemon refusal verbatim below the control', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(), 'POST /api/projects/p1/structure': () => Response.json(
      {code:'STAGE_BLOCKED',message:'reserved path: backend/CONTRACT.yaml'},{status:409}),
  })
  await until(() => a.button('Create structure')?.disabled === false)
  act(() => a.button('Create structure')?.click())
  await until(() => a.button('Confirm create structure')?.disabled === false)
  act(() => a.button('Confirm create structure')?.click())
  await until(() => a.text().includes('reserved path: backend/CONTRACT.yaml'))
  expect(a.button('Create structure')).toBeDefined()
})

it('shows Structure written with no create button at roadmap', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', answers('roadmap'))
  await until(() => a.text().includes('Structure written'))
  expect(a.button('Create structure')).toBeUndefined()
})

it('shows the count from the stage without reading the part tree', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', answers('structure', [], { revision: 4, count: 7 }))
  await until(() => a.button('Create structure')?.disabled === false)
  const partReads = a.calls.filter(c => c.includes('/design/parts')).length
  act(() => a.button('Create structure')?.click())
  await until(() => a.container.querySelector('[role="dialog"]')?.textContent?.includes('Write 7 files') === true)
  expect(a.calls.filter(c => c.includes('/design/parts'))).toHaveLength(partReads)
})

it('shows no count when the stage sends none, and still confirms', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', answers('structure', [], { revision: 1, count: null }))
  await until(() => a.button('Create structure')?.disabled === false)
  act(() => a.button('Create structure')?.click())
  await until(() => a.button('Confirm create structure')?.disabled === false)
  expect(a.container.querySelector('[role="dialog"]')!.textContent).not.toMatch(/\d+ files/)
})

it('re-reads the stage when the map moved and confirms again with the new revision', async () => {
  const map = { revision: 1, count: 3 as number | null }
  const a = app = await startApp('/projects/p1/workspace?view=map', answers('structure', [], map))
  await until(() => a.button('Create structure')?.disabled === false)
  act(() => a.button('Create structure')?.click())
  await until(() => a.button('Confirm create structure')?.disabled === false)
  map.revision = 2; map.count = 5
  act(() => a.button('Confirm create structure')?.click())
  await until(() => a.text().includes('The map changed; check the new count.'))
  await until(() => a.container.querySelector('[role="dialog"]')?.textContent?.includes('Write 5 files') === true)
  await until(() => a.button('Confirm create structure')?.disabled === false)
  expect(a.text()).not.toContain('Structure written')
  act(() => a.button('Confirm create structure')?.click())
  await until(() => a.text().includes('Structure written'))
  const sent = a.bodies.filter(b => (b as {map_revision?: number}).map_revision !== undefined) as {command_id: string; map_revision: number}[]
  expect(sent.map(b => b.map_revision)).toEqual([1, 2])
  expect(sent[0].command_id).not.toBe(sent[1].command_id)
})
