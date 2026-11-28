// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import type { VisionProposal } from '@/api/design'
import { visionProposalEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import { VisionProposalCard } from './vision-proposal-card'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; vi.restoreAllMocks() })
const pending = (): VisionProposal => ({ id: 'p1', thread_id: 't1', action: 'add', field: 'goals',
  item_id: null, text: 'Ship', old_text: null, expected_item_revision: null, status: 'Pending',
  outdated: false, created_at: '2026-10-12T00:00:00Z' })

it('approves the exact proposal and reads its resulting live status', async () => {
  expect(VisionProposalCard).toBeTypeOf('function')
  let live = pending()
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1', pending())] }),
    'GET /api/projects/p1/vision-proposals': () => Response.json([live]),
    'POST /api/projects/p1/vision-proposals/p1/approve': () => {
      live = { ...live, status: 'Approved', item_id: 'G1' }
      return Response.json({ revision: 1 })
    },
  })
  await until(() => a.button('Approve') !== undefined)
  expect(a.text()).toContain('Add goal')
  act(() => a.button('Approve')?.click())
  await until(() => a.text().includes('Approved · G1'))
  expect(a.button('Approve')).toBeUndefined()
  expect(a.calls).toContain('POST /api/projects/p1/vision-proposals/p1/approve')
  expect(a.bodies.at(-1)).toMatchObject({ command_id: expect.any(String) })
})

it.each([
  ['Superseded', false, 'Superseded'], ['Pending', true, 'Outdated — ask the Planner to propose again'],
  ['Approved', false, 'Approved · G1'],
] as const)('uses live %s state instead of the snapshot', async (status,outdated,label) => {
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1', pending())] }),
    'GET /api/projects/p1/vision-proposals': [{ ...pending(), status, outdated, item_id: 'G1' }],
  })
  await until(() => a.text().includes(label))
  expect(a.button('Approve')).toBeUndefined()
})

it.each(['edit','remove'] as const)('shows the old item for %s', async action => {
  const p = { ...pending(), action, item_id: 'G1', old_text: 'Old goal',
    text: action === 'edit' ? 'New goal' : null, expected_item_revision: 1 }
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1',p)] }),
    'GET /api/projects/p1/vision-proposals': [p],
  })
  await until(() => a.button('Approve') !== undefined)
  expect(a.text()).toContain('Old goal')
  expect(a.text()).toContain('G1')
  if (action === 'edit') expect(a.container.querySelector('del')?.textContent).toBe('Old goal')
})

it('shows refusal and reuses the command for an unchanged retry', async () => {
  let live = pending(), attempts = 0
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1', pending())] }),
    'GET /api/projects/p1/vision-proposals': () => Response.json([live]),
    'POST /api/projects/p1/vision-proposals/p1/approve': () => {
      if (++attempts === 1) return Response.json({ code:'INVALID_COMMAND', message:'please retry' },{status:422})
      live = {...live,status:'Approved',item_id:'G1'}
      return Response.json({revision:1})
    },
  })
  await until(() => a.button('Approve') !== undefined)
  act(() => a.button('Approve')?.click())
  await until(() => a.text().includes('please retry'))
  act(() => a.button('Approve')?.click())
  await until(() => a.text().includes('Approved · G1'))
  expect(a.bodies[0]).toEqual(a.bodies[1])
})

it('does not expose approval when live status cannot be read', async () => {
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1', pending())] }),
    'GET /api/projects/p1/vision-proposals': () => Response.json({code:'STORAGE_UNAVAILABLE',message:'status unavailable'},{status:503}),
  })
  await until(() => a.text().includes('status unavailable'))
  expect(a.button('Approve')).toBeUndefined()
})

it('waits for live status before exposing approval', async () => {
  let answer: ((response: Response) => void) | undefined
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [visionProposalEntry('e1',pending())] }),
    'GET /api/projects/p1/vision-proposals': () => new Promise<Response>(resolve => { answer = resolve }),
  })
  await until(() => a.text().includes('Loading status'))
  expect(a.button('Approve')).toBeUndefined()
  await act(async () => answer?.(Response.json([pending()])))
  await until(() => a.button('Approve') !== undefined)
})

it('copies the recorded body through the usual message actions', async () => {
  const writes: string[] = []
  vi.spyOn(navigator,'clipboard','get').mockReturnValue({writeText: async (t:string) => { writes.push(t) }} as Clipboard)
  const entry = visionProposalEntry('e1',pending())
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers({ entries: [entry] }),
    'GET /api/projects/p1/vision-proposals': [pending()],
  })
  await until(() => a.buttons('Copy').length > 0)
  await act(async () => a.buttons('Copy')[0]?.click())
  await until(() => writes.length === 1)
  expect(writes).toEqual([entry.body])
})
