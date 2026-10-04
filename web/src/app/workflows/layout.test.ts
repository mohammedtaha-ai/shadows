// The plan graph's automatic layout (§13.11): left to right, and never two
// boxes on top of each other, however many tasks the Planner writes.

import { describe, expect, it } from 'vitest'
import type { PlanLink } from '@/api/client'
import { planFixture, planTask } from '@/test/contract-fixtures'
import { layoutPlan } from './layout'

interface Box {
  id: string
  x: number
  y: number
  width: number
  height: number
}

function boxes(nodes: ReturnType<typeof layoutPlan>['nodes']): Map<string, Box> {
  return new Map(
    nodes.map((n) => [
      n.id,
      { id: n.id, x: n.position.x, y: n.position.y, width: n.width ?? 0, height: n.height ?? 0 },
    ]),
  )
}

describe('layoutPlan', () => {
  it('lays out left to right with each task right of what it needs', () => {
    const plan = planFixture({
      tasks: [planTask(1, 'Schema'), planTask(2, 'API'), planTask(3, 'Screen'), planTask(4, 'Docs')],
      links: [
        { task: 2, after: 1, kind: 'needs', label: 'tables', waiting_items: [] },
        { task: 3, after: 2, kind: 'needs', label: 'routes', waiting_items: [] },
        { task: 4, after: 1, kind: 'needs', label: 'names', waiting_items: [] },
      ],
    })

    const { nodes, edges } = layoutPlan(plan)
    const at = boxes(nodes)

    for (const link of plan.links) {
      const before = at.get(`t${link.after}`)
      const after = at.get(`t${link.task}`)
      if (before === undefined || after === undefined) throw new Error('a task has no node')
      expect(after.x).toBeGreaterThan(before.x + before.width)
    }
    // The start node is left of every task.
    const start = at.get('start')
    if (start === undefined) throw new Error('no start node')
    for (const task of plan.tasks) {
      expect(at.get(`t${task.number}`)?.x).toBeGreaterThan(start.x + start.width)
    }
    expect(edges.filter((e) => e.data?.kind === 'needs')).toHaveLength(3)
  })

  it('lays out 60 tasks without overlapping nodes', () => {
    const tasks = Array.from({ length: 60 }, (_, i) =>
      planTask(i + 1, `Task ${i + 1}`, { writes: i % 3 === 0 ? ['src/a.rs', 'src/b.rs'] : [] }),
    )
    // A fixed, irregular web of both kinds: no randomness, so every run
    // lays out the same plan.
    const links: PlanLink[] = []
    for (let n = 2; n <= 60; n += 1) {
      links.push({ task: n, after: n - 1 - (n % 4), kind: 'needs', label: `from ${n}`, waiting_items: [] })
      if (n % 5 === 0 && n > 8) {
        links.push({ task: n, after: n - 7, kind: 'completes_after', label: 'check', waiting_items: [1] })
      }
    }
    const { nodes } = layoutPlan(planFixture({ tasks, links: links.filter((l) => typeof l.after === 'number' && l.after >= 1) }))

    const all = [...boxes(nodes).values()]
    expect(all).toHaveLength(61)
    for (const [i, a] of all.entries()) {
      expect(a.width).toBeGreaterThan(0)
      expect(a.height).toBeGreaterThan(0)
      for (const b of all.slice(i + 1)) {
        const disjoint =
          a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y
        if (!disjoint) throw new Error(`${a.id} overlaps ${b.id}`)
      }
    }
  })
})

it('places outgoing and incoming tasks using plan-qualified node identities', () => {
  const outgoing = {
    link: { task: 4, after: { plan_id: 'backend', task: 3 }, kind: 'needs' as const, label: 'API', waiting_items: [] },
    incoming: false, plan_id: 'backend', project_id: 'other', project_name: 'Other project',
    workflow_id: 'backend-v2', version: 2, plan_title: 'Backend', plan_state: 'Active' as const,
    state: 'Draft' as const, task: { number: 3, title: 'Login API', goal: 'Authenticate', acceptance: [], state: 'Pending' },
    broken: null,
  }
  const incoming = { ...outgoing, incoming: true, plan_id: 'consumer', plan_title: 'Mobile',
    link: { ...outgoing.link, task: 3, after: { plan_id: 'plan1', task: 4 } } }
  const plan = planFixture({ tasks: [planTask(4, 'Login screen')], links: [outgoing.link], linked_tasks: [outgoing, incoming] })
  const { nodes, edges } = layoutPlan(plan)
  expect(nodes.filter(node => node.type === 'linked').map(node => node.id)).toEqual(['pbackend-t3', 'pconsumer-t3'])
  expect(edges.some(edge => edge.source === 'pbackend-t3' && edge.target === 't4')).toBe(true)
  expect(edges.some(edge => edge.source === 't4' && edge.target === 'pconsumer-t3')).toBe(true)
  expect(edges.some(edge => edge.id === 'start-4')).toBe(false)
})

it('keeps a broken dependency node with its explanation', () => {
  const link = { task: 4, after: { plan_id: 'backend', task: 3 }, kind: 'needs' as const, label: 'API', waiting_items: [] }
  const plan = planFixture({ tasks: [planTask(4, 'Screen')], links: [link], linked_tasks: [{
    link, incoming: false, plan_id: 'backend', project_id: 'other', project_name: 'Other',
    workflow_id: 'backend-v2', version: 2, plan_title: 'Backend', plan_state: 'Active', state: 'Draft',
    task: null, broken: 'T3 is missing from the latest version',
  }] })
  const { nodes, edges } = layoutPlan(plan)
  expect(nodes.find(node => node.id === 'pbackend-t3')?.data.view).toMatchObject({ broken: 'T3 is missing from the latest version' })
  expect(edges.find(edge => edge.source === 'pbackend-t3')?.data?.broken).toBe(true)
})

it('retains an incoming edge when its local target task is missing', () => {
  const plan = planFixture({ tasks: [planTask(5, 'Replacement API')], linked_tasks: [{
    link: { task: 4, after: { plan_id: 'plan1', task: 3 }, kind: 'needs', label: 'API', waiting_items: [] },
    incoming: true, plan_id: 'web', project_id: 'p2', project_name: 'Web project', workflow_id: 'web-v1',
    version: 1, plan_title: 'Web', plan_state: 'Active', state: 'Draft',
    task: { number: 4, title: 'Login screen', goal: 'Sign in', acceptance: [], state: 'Pending' },
    broken: 'T3 is missing from the latest version',
  }] })
  const { nodes, edges } = layoutPlan(plan)
  expect(nodes.find(node => node.id === 't3')).toMatchObject({ type: 'missing' })
  expect(nodes.some(node => node.id === 'pweb-t4')).toBe(true)
  expect(edges.find(edge => edge.source === 't3' && edge.target === 'pweb-t4')?.data?.broken).toBe(true)
})
