import { expect, it } from 'vitest'
import type { DesignElement, ElementView } from '@/api/project-state'
import { EDGE_LABEL, NODE_LINES, screenDiagram, tableDiagram } from './diagram-layout'

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
  expect(first.nodes.map(node => [node.id, node.width, node.height])).toEqual([['E1', 260, 96], ['E2', 260, 114]])
  for (const node of first.nodes) {
    expect(node.measured).toEqual({ width: node.width, height: node.height })
    expect(node.handles).toHaveLength(2)
  }
  expect(first.nodes[1].data.rows).toEqual(['id text · key', 'user_id text → users.id'])
  expect(first.edges.map(({ id, type, source, target, data }) => [id, type, source, target, data?.label]))
    .toEqual([['fk:E2:user_id', 'diagram', 'E2', 'E1', 'user_id → users.id']])
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
  expect(out.nodes.map(node => [node.id, node.width, node.height])).toEqual([['E1', 240, 132], ['E2', 240, 114]])
  expect(out.nodes[0].data.rows).toEqual(['/login', 'Login purpose', '→ E9 when lost'])
  expect(out.nodes[1].data.rows[0]).toBe('No route')
  expect(out.edges.map(({ id, source, target, data }) => [id, source, target, data?.label])).toEqual([['link:E1:0', 'E1', 'E2', 'signed in']])
})

it('a_node_is_as_tall_as_every_line_it_draws_for_any_number_of_rows', () => {
  const header = NODE_LINES.frame + NODE_LINES.name + NODE_LINES.meta + NODE_LINES.badge + NODE_LINES.gap
  for (const count of [0, 1, 4, 12]) {
    const columns = Array.from({ length: count }, (_, index) => ({ name: `c${index}` }))
    const [node] = tableDiagram([table('E1', 'people', columns)]).nodes
    expect(node.height).toBe(header + count * NODE_LINES.row)
    expect(node.measured).toEqual({ width: 260, height: node.height })
  }
  const behind = table('E1', 'people', [{ name: 'id' }])
  behind.state = { ...behind.state, state: 'built', behind: true, built_revision: 1 }
  expect(tableDiagram([behind]).nodes[0].height).toBe(header + NODE_LINES.behind + NODE_LINES.row)
  // Route and purpose are both rows of a screen.
  expect(screenDiagram([screen('E1', 'Sign in', [])]).nodes[0].height).toBe(header + 2 * NODE_LINES.row)
})

it('links_both_ways_draw_two_arrows_between_the_two_nodes_apart', () => {
  const out = screenDiagram([
    screen('E2', 'Sign in', [['E3', 'chooses to create an account']], '/sign-in'),
    screen('E3', 'Sign up', [['E2', 'chooses to sign in']], '/sign-up'),
  ])
  const [signIn, signUp] = out.nodes
  const [forward, back] = out.edges
  expect([forward.source, forward.target, back.source, back.target]).toEqual(['E2', 'E3', 'E3', 'E2'])
  const left = Math.min(signIn.position.x, signUp.position.x) + 240
  const right = Math.max(signIn.position.x, signUp.position.x)
  for (const edge of out.edges) {
    const data = edge.data!
    // The label sits in the gap between the two nodes, so the arrow is drawn there.
    expect(data.labelX - EDGE_LABEL.width / 2).toBeGreaterThanOrEqual(left)
    expect(data.labelX + EDGE_LABEL.width / 2).toBeLessThanOrEqual(right)
    // Every point of the path stays between the nodes' facing sides.
    const xs = [...data.path.matchAll(/(-?[\d.]+),(-?[\d.]+)/g)].map(match => Number(match[1]))
    expect(Math.min(...xs)).toBeGreaterThanOrEqual(left)
    expect(Math.max(...xs)).toBeLessThanOrEqual(right)
  }
  expect(forward.data!.path).not.toBe(back.data!.path)
  expect(Math.abs(forward.data!.labelY - back.data!.labelY)).toBeGreaterThanOrEqual(EDGE_LABEL.height)
  // Each starts on its source's side facing the other node and ends on its target's.
  expect(forward.data!.path.startsWith(`M${left},`)).toBe(true)
  expect(back.data!.path.startsWith(`M${right},`)).toBe(true)
})

it('two_foreign_keys_to_one_table_get_two_label_slots', () => {
  const out = tableDiagram([
    table('E1', 'users', [{ name: 'id' }]),
    table('E2', 'transfers', [{ name: 'from_id', ref: ['E1', 'id'] }, { name: 'to_id', ref: ['E1', 'id'] }]),
  ])
  expect(out.edges).toHaveLength(2)
  expect(Math.abs(out.edges[0].data!.labelY - out.edges[1].data!.labelY)).toBeGreaterThanOrEqual(EDGE_LABEL.height)
})
