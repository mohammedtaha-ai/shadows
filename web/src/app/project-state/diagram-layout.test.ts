import { expect, it } from 'vitest'
import type { DesignElement, ElementView } from '@/api/project-state'
import { screenDiagram, tableDiagram } from './diagram-layout'

const base = { revision: 1, project_id: 'p1', part_id: 'web', outcomes: [], deleted: false, written_by: 't1', created_at: '2026-11-01T00:00:00Z' }
const table = (id: string, name: string, columns: { name: string; ref?: [string, string] }[]): ElementView => {
  const element: DesignElement = { ...base, id, content: { kind: 'table', name, purpose: 'p', columns: columns.map(column => ({
    name: column.name, type_name: 'text', primary_key: column.name === 'id',
    references: column.ref === undefined ? null : { table: column.ref[0], column: column.ref[1] },
  })) } }
  return { element, state: { element_id: id, state: 'planned', behind: false, built_revision: null, done: [], planned: [] } }
}
const screen = (id: string, name: string, leads: [string, string][], route: string | null = null): ElementView => {
  const element: DesignElement = { ...base, id, content: { kind: 'screen', name, purpose: `${name} purpose`, route,
    leads_to: leads.map(([target, when]) => ({ screen: target, when })) } }
  return { element, state: { element_id: id, state: 'planned', behind: false, built_revision: null, done: [], planned: [] } }
}

it('the_layout_is_deterministic_and_every_node_has_its_size', () => {
  const views = [table('E1', 'users', [{ name: 'id' }]), table('E2', 'sessions', [{ name: 'id' }, { name: 'user_id', ref: ['E1', 'id'] }])]
  const first = tableDiagram(views)
  expect(tableDiagram(views)).toEqual(first)
  expect(first.nodes.map(node => [node.id, node.width, node.height])).toEqual([['E1', 260, 82], ['E2', 260, 100]])
  for (const node of first.nodes) {
    expect(node.measured).toEqual({ width: node.width, height: node.height })
    expect(node.handles).toHaveLength(2)
  }
  expect(first.nodes[1].data.rows).toEqual(['id text · key', 'user_id text → users.id'])
  expect(first.edges).toEqual([{ id: 'fk:E2:user_id', source: 'E2', target: 'E1', data: { label: 'user_id → users.id' } }])
})

it('a_reference_to_a_table_not_shown_draws_no_edge_but_keeps_its_row', () => {
  const out = tableDiagram([table('E2', 'sessions', [{ name: 'user_id', ref: ['E1', 'id'] }])])
  expect(out.edges).toEqual([])
  expect(out.nodes[0].data.rows).toEqual(['user_id text → E1.id'])
})

it('a_self_reference_draws_no_edge', () => {
  const out = tableDiagram([table('E1', 'nodes', [{ name: 'parent_id', ref: ['E1', 'id'] }])])
  expect(out.edges).toEqual([])
  expect(out.nodes[0].data.rows).toEqual(['parent_id text → nodes.id'])
})

it('screen_links_become_edges_labelled_with_when', () => {
  const out = screenDiagram([
    screen('E1', 'Login', [['E2', 'signed in'], ['E9', 'lost'], ['E1', 'again']], '/login'),
    screen('E2', 'Home', []),
  ])
  expect(out.nodes.map(node => [node.id, node.width, node.height])).toEqual([['E1', 240, 118], ['E2', 240, 100]])
  expect(out.nodes[0].data.rows).toEqual(['/login', 'Login purpose', '→ E9 when lost'])
  expect(out.nodes[1].data.rows[0]).toBe('No route')
  expect(out.edges).toEqual([{ id: 'link:E1:0', source: 'E1', target: 'E2', data: { label: 'signed in' } }])
})
