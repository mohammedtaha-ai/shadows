interface Fixture {
  name: string
  sources: Record<string, string>
  rule: string
  count?: number
  overrides?: Record<string, { contract?: string; entries?: string[] }>
}

const target = { 'app/agents/private.ts': 'export interface Shape {}; export const value = 1' }

export const fixtures: Fixture[] = [
  ...[
    ['public entry import', "import { isDesktop } from '@/desktop'"],
    ['startup import', "import { isDesktop } from '@/desktop/startup'"],
    ['runtime import alias', "import { isDesktop as platform } from '@/desktop/runtime'"],
    ['relative runtime import', "import { isDesktop } from '../../desktop/runtime'"],
    ['namespace call', "import * as native from '@/desktop/runtime'; native.isDesktop()"],
    ['aliased call', "import * as native from '@/desktop/runtime'; const platform = native.isDesktop; platform()"],
    ['local call', 'declare function isDesktop(): boolean; isDesktop()'],
  ].map(([name, source]) => ({
    name: `rejects desktop platform ${name} outside desktop`, rule: 'desktop_platform',
    sources: {
      'desktop/runtime.ts': 'export function isDesktop() { return true }',
      'desktop/index.ts': "export { isDesktop } from './runtime'",
      'desktop/startup.ts': "export { isDesktop } from './runtime'",
      'app/settings/view.ts': source!,
    },
  })),
  { name: 'allows desktop platform detection in its owner and tests', rule: 'desktop_platform', count: 0,
    sources: {
      'desktop/runtime.ts': 'export function isDesktop() { return true }',
      'desktop/capabilities.ts': "import { isDesktop } from './runtime'; export const enabled = () => isDesktop()",
      'app/settings/view.test.ts': "import { isDesktop } from '@/desktop/runtime'; isDesktop()",
    },
  },
  ...['desktop/startup.ts', 'app/desktop/startup.ts'].map(entry => ({
    name: `rejects API initialization through ${entry}`,
    rule: 'startup_api',
    overrides: { [entry === 'desktop/startup.ts' ? 'desktop' : 'desktop-ui']: { entries: [entry] } },
    sources: {
      [entry]: "export { value } from '@/desktop/unsafe'",
      'desktop/unsafe.ts': "import { value } from '@/api/transport/index'; export { value }",
      'api/transport/index.ts': 'export const value = 1',
    },
  })),
  { name: 'rejects API initialization through main.tsx static imports', rule: 'startup_api', sources: {
      'main.tsx': "import { value } from './app-root'; void value",
      'app-root.tsx': "export { value } from '@/api/transport/index'",
      'api/transport/index.ts': 'export const value = 1',
    },
  },
  { name: 'allows deferred and type-only API references from startup', rule: 'startup_api', count: 0,
    overrides: { desktop: { entries: ['desktop/startup.ts'] } }, sources: {
      'desktop/startup.ts': "import type { Shape } from '@/api/transport/index'; export const load = () => import('@/api/transport/index')",
      'api/transport/index.ts': 'export interface Shape {}',
    },
  },
  ...[
    ['alias import', "import { value } from '@/app/agents/private'"],
    ['relative import', "import { value } from '../agents/private'"],
    ['type-only import', "import type { Shape } from '../agents/private'"],
    ['inline type-only import', "import { type Shape } from '../agents/private'"],
    ['re-export', "export { value } from '../agents/private'"],
    ['star re-export', "export * from '../agents/private'"],
    ['type re-export', "export type { Shape } from '../agents/private'"],
    ['literal dynamic import', "void import('@/app/agents/private')"],
    ['import type expression', "type X = import('../agents/private').Shape"],
  ].map(([name, source]) => ({ name: `rejects private ${name}`, sources: { ...target, 'app/settings/view.ts': source! }, rule: 'public_entry' })),
  { name: 'rejects a type-only upward dependency', rule: 'dependency', sources: {
    'desktop/up.ts': "import type { Shape } from '@/app/agents/private'", ...target,
  } },
  { name: 'checks feature tests without a blanket exemption', rule: 'dependency', sources: {
    'app/design/view.test.ts': "import { value } from '../agents/private'", ...target,
  } },
  { name: 'rejects a runtime file cycle', rule: 'runtime_cycle', count: 2, sources: {
    'app/agents/a.ts': "import './b'", 'app/agents/b.ts': "export * from './a'",
  } },
  { name: 'does not turn type-only edges into runtime cycles', rule: 'runtime_cycle', count: 0, sources: {
    'app/agents/a.ts': "import type { B } from './b'; export interface A {}", 'app/agents/b.ts': "import './a'; export interface B {}",
  } },
  ...[
    ['global fetch', "fetch('/api/projects')"],
    ['globalThis fetch', "globalThis.fetch('/api/projects')"],
    ['globalThis fetch alias', "const send = globalThis.fetch; send('/api/projects')"],
    ['window fetch', "window.fetch('/api/projects')"],
    ['element fetch', "globalThis['fetch']('/api/projects')"],
    ['destructured fetch', "const { fetch: send } = window; send('/api/projects')"],
    ['bound fetch', "const send = fetch.bind(window); send('/api/projects')"],
    ['EventSource', "new EventSource('/api/events')"],
    ['EventSource alias', "const Events = EventSource; new Events('/api/events')"],
    ['object fetch alias', "const network = { send: globalThis.fetch }; network.send('/api/projects')"],
    ['shorthand fetch alias', "const network = { fetch }; network.fetch('/api/projects')"],
    ['generated client factory', "import makeClient from 'openapi-fetch'; const client = makeClient()"],
    ['generated client access', "import { client } from '@/api/client'; client.GET('/api/projects')"],
  ].map(([name, source]) => ({ name: `rejects foreign ${name}`, rule: 'server_access', sources: {
    'api/client.ts': "import createClient from 'openapi-fetch'; export const client = createClient()",
    'app/agents/view.ts': source!,
  } })),
  { name: 'allows the injected fetch callback in RulePreviewGuard', rule: 'server_access', count: 0, sources: {
    'app/desktop/rule-preview.ts': 'export class RulePreviewGuard { async load(rule: string, fetch: (rule: string) => Promise<unknown>) { return await fetch(rule) } }',
  } },
  { name: 'allows network calls in their named owners', rule: 'server_access', count: 0, sources: {
    'api/client.ts': "fetch('/api/projects')", 'stream/events.ts': "new EventSource('/api/events')",
  } },
  { name: 'rejects literal query identities', rule: 'query_key', sources: {
    'app/agents/view.ts': "useQuery({ queryKey: ['projects'] })",
  } },
  { name: 'rejects a locally returned query identity', rule: 'query_key', sources: {
    'app/agents/view.ts': "function key() { return ['projects'] }; useQuery({ queryKey: key() })",
  } },
  { name: 'rejects a locally aliased query identity', rule: 'query_key', sources: {
    'app/agents/view.ts': "const key = ['projects'] as const; useQuery({ queryKey: key })",
  } },
  { name: 'rejects a query identity spread from an API key', rule: 'query_key', sources: {
    'api/keys.ts': "export const key = ['projects']",
    'app/agents/view.ts': "import { key } from '@/api/keys'; useQuery({ queryKey: [...key, 'local'] })",
  } },
  { name: 'rejects a query identity stored in an object', rule: 'query_key', sources: {
    'app/agents/view.ts': "const keys = { projects: ['projects'] }; useQuery({ queryKey: keys.projects })",
  } },
  { name: 'rejects a destructured query identity', rule: 'query_key', sources: {
    'app/agents/view.ts': "const keys = { projects: ['projects'] }; const { projects: key } = keys; useQuery({ queryKey: key })",
  } },
  { name: 'rejects a query identity stored as a shorthand property', rule: 'query_key', sources: {
    'app/agents/view.ts': "const projects = ['projects']; const keys = { projects }; useQuery({ queryKey: keys.projects })",
  } },
  { name: 'allows API-owned query identities', rule: 'query_key', count: 0, sources: {
    'api/keys.ts': "export const key = () => ['projects']", 'app/agents/view.ts': "import { key } from '@/api/keys'; useQuery({ queryKey: key() })",
  } },
  { name: 'rejects a cache write outside API', rule: 'cache_write', sources: {
    'app/agents/view.ts': "client.setQueryData(key, answer)",
  } },
  { name: 'checks cache writes in tests too', rule: 'cache_write', sources: {
    'app/agents/view.test.ts': "client['setQueryData'](key, answer)",
  } },
  { name: 'rejects aliased cache writes', rule: 'cache_write', sources: {
    'app/agents/view.ts': 'const save = client.setQueryData; save(key, answer)',
  } },
  { name: 'rejects destructured cache writes', rule: 'cache_write', sources: {
    'app/agents/view.ts': 'const { setQueryData: save } = client; save(key, answer)',
  } },
  { name: 'rejects bound renamed cache writes', rule: 'cache_write', sources: {
    'app/agents/view.ts': 'const cache = queryClient; const save = cache.setQueryData.bind(cache); save(key, answer)',
  } },
  { name: 'rejects cache writes stored in an object', rule: 'cache_write', sources: {
    'app/agents/view.ts': 'const cache = { save: queryClient.setQueryData }; cache.save(key, answer)',
  } },
  { name: 'rejects cache writes stored as a shorthand property', rule: 'cache_write', sources: {
    'app/agents/view.ts': 'const save = queryClient.setQueryData; const cache = { save }; cache.save(key, answer)',
  } },
  { name: 'rejects shorthand query identities', rule: 'query_key', sources: {
    'app/agents/view.ts': "const queryKey = ['projects']; useQuery({ queryKey })",
  } },
  { name: 'allows API-owned cache writes', rule: 'cache_write', count: 0, sources: {
    'api/keys.ts': "client.setQueryData(['projects'], answer)",
  } },
  { name: 'rejects transitive production reachability to fixtures', rule: 'production_test', sources: {
    'main.tsx': "import './app/shell'", 'app/shell.tsx': "void import('@/test/fake')", 'test/fake.ts': 'export {}',
  } },
  { name: 'rejects production reachability through a product re-export', rule: 'production_test', sources: {
    'main.tsx': "import './app/agents'", 'app/agents/index.ts': "export * from '@/test/fake'", 'test/fake.ts': 'export const fake = 1',
  } },
  { name: 'allows the named test-support import exemption', rule: 'dependency', count: 0, sources: {
    'app/agents/view.test.ts': "import '@/test/fake'", 'test/fake.ts': 'export {}',
  } },
  { name: 'rejects an unowned source', rule: 'map', sources: { 'unknown/file.ts': 'export {}' } },
  { name: 'rejects an unresolved local edge', rule: 'map', sources: { 'app/agents/view.ts': "import '@/app/agents/missing'" } },
  { name: 'rejects invalid YAML contracts', rule: 'contract', overrides: { agents: { contract: 'app/agents/contract.yaml' } }, sources: {
    'app/agents/contract.yaml': 'name: { broken',
  } },
  { name: 'rejects missing contracts', rule: 'contract', overrides: { agents: { contract: 'app/agents/contract.yaml' } }, sources: {} },
  { name: 'rejects contracts identifying a different folder', rule: 'contract', overrides: { agents: { contract: 'app/agents/contract.yaml' } }, sources: {
    'app/agents/contract.yaml': 'name: agents\nversion: 1\nsource: web/src/api',
  } },
  { name: 'rejects missing public entries', rule: 'entry', overrides: { agents: { entries: ['app/agents/index.ts'] } }, sources: {} },
  { name: 'rejects a public entry assigned to another owner', rule: 'map', overrides: { agents: { entries: ['app/design/index.ts'] } }, sources: {
    'app/design/index.ts': 'export {}',
  } },
  { name: 'allows a declared public edge in the approved graph', rule: 'public_entry', count: 0, overrides: { agents: { entries: ['app/agents/index.ts'] } }, sources: {
    'app/settings/view.ts': "import { value } from '../agents'", 'app/agents/index.ts': 'export const value = 1',
  } },
  { name: 'rejects a feature import of the API transport', rule: 'public_entry', sources: {
    'app/settings/view.ts': "import { client } from '@/api/transport'; void client", 'api/transport/index.ts': 'export const client = 1',
  } },
  { name: 'does not confuse a shadowed window with the global object', rule: 'server_access', count: 0, sources: {
    'app/agents/view.ts': "function load(window: { fetch: () => void }) { window.fetch() }",
  } },
]
