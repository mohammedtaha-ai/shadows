// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, useEffect } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import { approveVisionProposal, visionProposalsQuery } from '@/api/design'
import { useProposalApproval } from './use-proposal-approval'
import type { RoadmapProposal, VisionProposal } from '@/api/design'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const connection = (id: string, name: string | null) => ({ id, name, project_ids: ['p1'],
  project_revision: 1, created_at: '2026-10-12T00:00:00Z' })
const external = (over: Partial<VisionProposal> = {}): VisionProposal => ({ id: 'x1', thread_id: null,
  connection_id: 'c1', grant_id: 'g1', action: 'add', field: 'goals', item_id: null, text: 'Ship it',
  old_text: null, expected_item_revision: null, status: 'Pending', outdated: false,
  created_at: '2026-10-12T00:00:00Z', ...over })
const vision = { 'GET /api/projects/p1/design/vision': { revision: 0, content: {
  purpose: [], users: [], goals: [], boundaries: [], technical_direction: [] } } }

it('an_external_vision_proposal_is_approved_on_the_vision_page', async () => {
  let live = external()
  const a = app = await startApp('/projects/p1/workspace', { ...answers(), ...vision,
    'GET /api/external-connections': [connection('c1', 'Codex')],
    'GET /api/projects/p1/vision-proposals': () => Response.json([live]),
    'POST /api/projects/p1/vision-proposals/x1/approve': () => {
      live = { ...live, status: 'Approved', item_id: 'G1' }
      return Response.json({ revision: 1 })
    },
  })
  await until(() => a.button('Approve') !== undefined)
  expect(a.text()).toContain('Ship it')
  act(() => a.button('Approve')?.click())
  // An approved proposal leaves the section: its page now shows the item itself.
  await until(() => !a.text().includes('Ship it'))
  expect(a.calls.filter(c => c === 'POST /api/projects/p1/vision-proposals/x1/approve')).toHaveLength(1)
})

it('a_retry_of_an_external_approval_reuses_its_command_id', async () => {
  let live = external(), attempts = 0
  const a = app = await startApp('/projects/p1/workspace', { ...answers(), ...vision,
    'GET /api/projects/p1/vision-proposals': () => Response.json([live]),
    'POST /api/projects/p1/vision-proposals/x1/approve': () => {
      if (++attempts === 1) return Response.json({ code: 'INVALID_COMMAND', message: 'please retry' }, { status: 422 })
      live = { ...live, status: 'Approved', item_id: 'G1' }
      return Response.json({ revision: 1 })
    },
  })
  await until(() => a.button('Approve') !== undefined)
  act(() => a.button('Approve')?.click())
  await until(() => a.text().includes('please retry'))
  act(() => a.button('Approve')?.click())
  await until(() => !a.text().includes('Ship it'))
  expect(a.bodies[0]).toEqual(a.bodies[1])
})

it('an_external_proposal_names_its_connection', async () => {
  const a = app = await startApp('/projects/p1/workspace', { ...answers(), ...vision,
    'GET /api/external-connections': [connection('c1', 'Codex')],
    'GET /api/projects/p1/vision-proposals': [external()],
  })
  await until(() => a.text().includes('From Codex'))
  expect(a.text()).toContain('From external agents')
})

it('an_external_proposal_of_a_removed_connection_says_an_external_agent', async () => {
  const a = app = await startApp('/projects/p1/workspace', { ...answers(), ...vision,
    'GET /api/projects/p1/vision-proposals': [external({ connection_id: 'gone' })],
  })
  await until(() => a.text().includes('From an external agent'))
})

it('the_pending_aside_links_only_proposals_with_a_conversation', async () => {
  const a = app = await startApp('/projects/p1/workspace', { ...answers(), ...vision,
    'GET /api/projects/p1/vision-proposals': [external(), external({ id: 'v2', thread_id: 't1' })],
  })
  await until(() => a.text().includes('1 proposal awaits your approval'))
  expect(a.text()).toContain('From an external agent')
  expect(a.container.querySelectorAll('aside[role="status"] a')).toHaveLength(1)
})

it('the_roadmap_page_lists_external_roadmap_proposals', async () => {
  const p: RoadmapProposal = { id: 'r1', thread_id: null, connection_id: 'c1', grant_id: 'g1',
    action: 'first', old: null, status: 'Pending', outdated: false, created_at: '2026-10-12T00:00:00Z',
    outcomes: [{ key: 'login', parent_key: null, parent: null, parts: [],
      content: { title: 'Login works', intended_result: 'People sign in', acceptance: ['It signs in'] } }] }
  const a = app = await startApp('/projects/p1/workspace?view=roadmap', { ...answers(),
    'GET /api/external-connections': [connection('c1', 'Codex')],
    'GET /api/projects/p1/roadmap-proposals': [p],
    'GET /api/projects/p1/design/outcomes': { revision: 1, items: [], next: null },
  })
  await until(() => a.button('Approve roadmap') !== undefined)
  expect(a.text()).toContain('From Codex')
  expect(a.text()).toContain('Login works')
})

it('an_outdated_external_proposal_asks_its_agent', async () => {
  let live = external()
  vi.stubGlobal('fetch', async () => Response.json([live]))
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const status = { value: '' }
  function Probe() {
    const approval = useProposalApproval({ projectId: 'p1', proposalId: 'x1',
      proposalsQuery: visionProposalsQuery('p1'), approveProposal: approveVisionProposal, external: true })
    useEffect(() => { status.value = approval.status })
    return null
  }
  const root = createRoot(document.body.appendChild(document.createElement('div')))
  await act(async () => { root.render(<QueryClientProvider client={queries}><Probe /></QueryClientProvider>) })
  await until(() => status.value === 'Pending')
  live = { ...live, outdated: true }
  await act(async () => { await queries.invalidateQueries() })
  await until(() => status.value === 'Outdated — ask its agent to propose again')
  act(() => root.unmount())
  vi.unstubAllGlobals()
})
