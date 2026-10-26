/// <reference types="node" />
import { readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'
import ts from 'typescript'
import { parseDocument } from 'yaml'
import { describe, expect, it } from 'vitest'
import { fixtures } from './architecture.fixtures'

type Sources = Record<string, string>
interface Owner {
  name: string
  sources: string[]
  entries: string[]
  contract: string
  allowed: string[]
}
interface Edge { from: string; to: string; runtime: boolean; static: boolean }
const src = import.meta.dirname.replaceAll('\\', '/')
const web = path.dirname(src)
const map = readFileSync(path.resolve(web, '../docs/frontend/codebase/README.md'), 'utf8')
const config = ts.readConfigFile(path.join(web, 'tsconfig.app.json'), ts.sys.readFile)
const options = ts.parseJsonConfigFileContent(config.config, ts.sys, web).options

function moduleTable(text: string): Owner[] {
  const section = text.split('## Architecture modules\n')[1]?.split('\n## ')[0] ?? ''
  return section.split('\n').filter(line => line.startsWith('| `')).map(line => {
    const cells = line.split('|').slice(1, -1).map(cell => cell.trim().replaceAll('`', ''))
    const list = (cell: string) => cell === '-' ? [] : cell.split(', ')
    return { name: cells[0]!, sources: list(cells[1]!), entries: list(cells[2]!),
      contract: cells[3]!, allowed: list(cells[4]!) }
  })
}

function matches(file: string, selector: string): boolean {
  if (selector.endsWith('/**')) return file.startsWith(selector.slice(0, -2))
  if (selector.endsWith('/*')) return path.posix.dirname(file) === selector.slice(0, -2)
  return file === selector
}

function walk(dir: string): Sources {
  return Object.assign({}, ...readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0).map(entry => {
    const full = path.join(dir, entry.name)
    return entry.isDirectory() ? walk(full) : /\.(tsx?|yaml)$/.test(entry.name)
      ? { [path.relative(src, full).replaceAll('\\', '/')]: readFileSync(full, 'utf8') } : {}
  })) as Sources
}

/** The same resolver/checker sees disk sources and the in-memory rejection cases. */
function scan(sources: Sources, owners: Owner[]): string[] {
  const findings: string[] = []
  const occurrences = new Map<string, number>()
  const add = (rule: string, file: string, target: string) => {
    const identity = `${rule} :: ${file} :: ${target}`
    const ordinal = (occurrences.get(identity) ?? 0) + 1
    occurrences.set(identity, ordinal)
    findings.push(`${identity} #${ordinal}`)
  }
  const absolute = (file: string) => `${src}/${file}`
  const relative = (file: string) => path.posix.relative(src, file.replaceAll('\\', '/'))
  const files = new Map(Object.entries(sources).map(([file, text]) => [absolute(file), text]))
  const host = ts.createCompilerHost(options)
  const diskRead = host.readFile.bind(host)
  const localSource = (file: string) => file.replaceAll('\\', '/').startsWith(`${src}/`)
  host.readFile = file => localSource(file) ? files.get(file.replaceAll('\\', '/')) : diskRead(file)
  host.fileExists = file => localSource(file) ? files.has(file.replaceAll('\\', '/')) : ts.sys.fileExists(file)
  // Virtual directories must participate in TypeScript's alias/relative resolution.
  host.directoryExists = dir => localSource(`${dir}/`) ? [...files.keys()].some(file => file.startsWith(`${dir.replaceAll('\\', '/')}/`)) : ts.sys.directoryExists(dir)
  const sourceNames = Object.keys(sources).filter(file => /\.tsx?$/.test(file)).sort()
  const program = ts.createProgram(sourceNames.map(absolute), options, host)
  const checker = program.getTypeChecker()
  const ownerOf = (file: string) => {
    const scored = owners.map(owner => ({ owner, score: Math.max(-1, ...owner.sources.filter(selector => matches(file, selector)).map(selector => selector.includes('*') ? selector.length : 10_000 + selector.length)) }))
    const best = Math.max(...scored.map(item => item.score))
    return scored.filter(item => item.score >= 0 && item.score === best).map(item => item.owner)
  }
  const ownerCache = new Map<string, Owner | undefined>()
  const owner = (file: string) => {
    if (!ownerCache.has(file)) ownerCache.set(file, ownerOf(file)[0])
    return ownerCache.get(file)
  }
  const edges: Edge[] = []

  for (const file of Object.keys(sources).filter(file => file.endsWith('/contract.yaml')).sort()) {
    if (!owners.some(item => item.contract === file)) add('map', file, 'unmapped contract')
  }

  for (const item of owners) {
    for (const dependency of item.allowed) {
      if (!owners.some(other => other.name === dependency)) add('map', item.name, `unknown dependency ${dependency}`)
    }
    for (const entry of item.entries) {
      if (entry.includes('*')) {
        if (!sourceNames.some(file => matches(file, entry) && owner(file)?.name === item.name)) add('entry', item.name, entry)
        continue
      }
      if (!(entry in sources)) add('entry', item.name, entry)
      else if (owner(entry)?.name !== item.name) add('map', item.name, `foreign entry ${entry}`)
    }
    if (item.contract !== '-') {
      const text = sources[item.contract]
      if (text === undefined) add('contract', item.name, `missing ${item.contract}`)
      else {
        const document = parseDocument(text, { uniqueKeys: true })
        if (document.errors.length) add('contract', item.name, `invalid ${item.contract}`)
        else {
          const value = document.toJS() as Record<string, unknown> | null
          const folder = item.contract.slice(0, -'/contract.yaml'.length)
          if (!value || !value.name || !value.version || value.source !== `web/src/${folder}`) {
            add('contract', item.name, `identity ${item.contract}`)
          }
        }
      }
    }
  }

  function valueSymbol(node: ts.Node): ts.Symbol | undefined {
    let symbol = checker.getSymbolAtLocation(node)
    if (symbol?.valueDeclaration && ts.isShorthandPropertyAssignment(symbol.valueDeclaration)) {
      symbol = checker.getShorthandAssignmentValueSymbol(symbol.valueDeclaration)
    }
    return symbol && symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol
  }

  /** Follow stored values so an object property cannot launder a forbidden operation. */
  function assignedExpression(declaration: ts.Declaration): ts.Expression | undefined {
    if (ts.isVariableDeclaration(declaration) || ts.isPropertyAssignment(declaration)) return declaration.initializer
    if (ts.isBindingElement(declaration) && ts.isVariableDeclaration(declaration.parent.parent)) {
      const container = declaration.parent.parent.initializer
      const name = declaration.propertyName ?? declaration.name
      if (container && (ts.isIdentifier(name) || ts.isStringLiteralLike(name))) {
        const property = checker.getTypeAtLocation(container).getProperty(name.text)?.valueDeclaration
        if (property && ts.isPropertyAssignment(property)) return property.initializer
      }
    }
    return undefined
  }

  /** Symbol provenance prevents a parameter named fetch from becoming a network call. */
  function origin(node: ts.Node, seen = new Set<ts.Symbol>()): string {
    if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isNonNullExpression(node)) return origin(node.expression, seen)
    if (ts.isCallExpression(node) || ts.isNewExpression(node)) return origin(node.expression, seen)
    if (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
      const name = ts.isPropertyAccessExpression(node) ? node.name.text : node.argumentExpression && ts.isStringLiteralLike(node.argumentExpression) ? node.argumentExpression.text : ''
      const root = origin(node.expression, new Set(seen))
      if (['bind', 'call', 'apply'].includes(name)) return root
      if (root === 'global' && ['fetch', 'EventSource'].includes(name)) return `global:${name}`
      if (root === 'openapi-fetch' && ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS', 'default'].includes(name)) return 'openapi-fetch'
    }
    const symbol = valueSymbol(node)
    if (!symbol || seen.has(symbol)) return ''
    seen.add(symbol)
    if (symbol.name === 'globalThis' && !symbol.declarations?.length) return 'global'
    for (const declaration of symbol.declarations ?? []) {
      const file = declaration.getSourceFile().fileName.replaceAll('\\', '/')
      if (file.includes('/typescript/lib/lib.')) {
        if (['fetch', 'EventSource'].includes(symbol.name)) return `global:${symbol.name}`
        if (['globalThis', 'window', 'self'].includes(symbol.name)) return 'global'
      }
      if (file.includes('/openapi-fetch/')) return 'openapi-fetch'
      const assigned = assignedExpression(declaration)
      if (assigned) return origin(assigned, seen)
      if (ts.isBindingElement(declaration) && ts.isVariableDeclaration(declaration.parent.parent) && declaration.parent.parent.initializer) {
        const root = origin(declaration.parent.parent.initializer, seen)
        const name = declaration.propertyName?.getText() ?? declaration.name.getText()
        if (root === 'global') return `global:${name}`
        return root
      }
    }
    return ''
  }

  function calledName(node: ts.Node, seen = new Set<ts.Symbol>()): string {
    if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isNonNullExpression(node)) return calledName(node.expression, seen)
    if (ts.isCallExpression(node)) return calledName(node.expression, seen)
    if (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
      const symbol = valueSymbol(node)
      const assigned = symbol?.valueDeclaration && assignedExpression(symbol.valueDeclaration)
      if (symbol && assigned) {
        if (seen.has(symbol)) return ''
        seen.add(symbol)
        return calledName(assigned, seen)
      }
      const name = ts.isPropertyAccessExpression(node) ? node.name.text : ts.isStringLiteralLike(node.argumentExpression) ? node.argumentExpression.text : ''
      return ['bind', 'call', 'apply'].includes(name) ? calledName(node.expression, seen) : name
    }
    const symbol = valueSymbol(node)
    if (!symbol || seen.has(symbol)) return node.getText()
    seen.add(symbol)
    for (const declaration of symbol.declarations ?? []) {
      const assigned = assignedExpression(declaration)
      if (assigned) return calledName(assigned, seen)
      if (ts.isBindingElement(declaration)) return (declaration.propertyName ?? declaration.name).getText()
    }
    return symbol.name
  }

  function hasLocalArray(node: ts.Node, seen = new Set<ts.Symbol>()): boolean {
    if (ts.isArrayLiteralExpression(node)) return true
    if (ts.isAsExpression(node) || ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node)) return hasLocalArray(node.expression, seen)
    if (ts.isArrowFunction(node)) {
      if (!ts.isBlock(node.body)) return hasLocalArray(node.body, seen)
      return node.body.statements.some(statement => ts.isReturnStatement(statement) && statement.expression && hasLocalArray(statement.expression, seen))
    }
    if (ts.isIdentifier(node) || ts.isCallExpression(node) || ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
      const symbol = valueSymbol(ts.isCallExpression(node) ? node.expression : node)
      if (!symbol || seen.has(symbol)) return false
      seen.add(symbol)
      return (symbol.declarations ?? []).some(declaration => {
        if (owner(relative(declaration.getSourceFile().fileName))?.name === 'api') return false
        const assigned = assignedExpression(declaration)
        if (assigned) return hasLocalArray(assigned, seen)
        if (ts.isFunctionDeclaration(declaration) || ts.isArrowFunction(declaration)) {
          let found = false
          const returns = (child: ts.Node) => {
            if (ts.isReturnStatement(child) && child.expression && hasLocalArray(child.expression, seen)) found = true
            ts.forEachChild(child, returns)
          }
          if (declaration.body) {
            if (ts.isArrayLiteralExpression(declaration.body)) return true
            returns(declaration.body)
          }
          return found
        }
        return false
      })
    }
    return false
  }

  for (const file of sourceNames) {
    const memberships = ownerOf(file)
    if (memberships.length !== 1) add('map', file, `owner count ${memberships.length}`)
    const from = owner(file)
    const tree = program.getSourceFile(absolute(file))!
    const edge = (literal: ts.StringLiteralLike, runtime: boolean, isStatic = true) => {
      const specifier = literal.text
      const resolved = ts.resolveModuleName(specifier, absolute(file), options, host).resolvedModule
      if (!resolved) {
        if (/^(\.|@\/)/.test(specifier) && !/\.(css|json)$/.test(specifier)) add('map', file, `unresolved ${specifier}`)
        return
      }
      const to = relative(resolved.resolvedFileName)
      if (!(to in sources)) return
      edges.push({ from: file, to, runtime, static: isStatic })
      const target = owner(to)
      if (!from || !target || from.name === target.name) return
      // Named exemption: tests may consume test infrastructure, without bypassing other rules.
      const fixtureImport = target.name === 'test-support' && target.entries.some(entry => matches(to, entry)) && (/\.test\.tsx?$/.test(file) || from.name === 'test-support')
      if (fixtureImport) return
      if (!target.entries.some(entry => matches(to, entry))) add('public_entry', file, to)
      if (!from.allowed.includes(target.name)) add('dependency', file, to)
    }
    const platformRestricted = !file.startsWith('desktop/') && !/\.test\.tsx?$/.test(file)
    const desktopImport = (literal: ts.StringLiteralLike) => {
      const resolved = ts.resolveModuleName(literal.text, absolute(file), options, host).resolvedModule
      return /^@\/desktop(?:\/|$)/.test(literal.text) ||
        (resolved !== undefined && relative(resolved.resolvedFileName).startsWith('desktop/'))
    }
    const visit = (node: ts.Node) => {
      if (platformRestricted) {
        if (ts.isImportDeclaration(node) && ts.isStringLiteralLike(node.moduleSpecifier) && desktopImport(node.moduleSpecifier)) {
          const bindings = node.importClause?.namedBindings
          if (bindings && ts.isNamedImports(bindings)) {
            for (const item of bindings.elements) {
              if ((item.propertyName ?? item.name).text === 'isDesktop') add('desktop_platform', file, 'import isDesktop')
            }
          }
        }
        if (ts.isExportDeclaration(node) && node.moduleSpecifier && ts.isStringLiteralLike(node.moduleSpecifier) && desktopImport(node.moduleSpecifier) && node.exportClause && ts.isNamedExports(node.exportClause)) {
          for (const item of node.exportClause.elements) {
            if ((item.propertyName ?? item.name).text === 'isDesktop') add('desktop_platform', file, 'export isDesktop')
          }
        }
        if (ts.isCallExpression(node) && (calledName(node.expression) === 'isDesktop' || (ts.isIdentifier(node.expression) && node.expression.text === 'isDesktop'))) {
          add('desktop_platform', file, 'call isDesktop')
        }
      }
      if (ts.isImportDeclaration(node) && ts.isStringLiteralLike(node.moduleSpecifier)) {
        const clause = node.importClause
        const allTypes = clause?.isTypeOnly || (!clause?.name && clause?.namedBindings && ts.isNamedImports(clause.namedBindings) && clause.namedBindings.elements.length > 0 && clause.namedBindings.elements.every(item => item.isTypeOnly))
        edge(node.moduleSpecifier, !allTypes)
      }
      if (ts.isExportDeclaration(node) && node.moduleSpecifier && ts.isStringLiteralLike(node.moduleSpecifier)) {
        const allTypes = node.isTypeOnly || (node.exportClause && ts.isNamedExports(node.exportClause) && node.exportClause.elements.length > 0 && node.exportClause.elements.every(item => item.isTypeOnly))
        edge(node.moduleSpecifier, !allTypes)
      }
      if (ts.isImportTypeNode(node) && ts.isLiteralTypeNode(node.argument) && ts.isStringLiteralLike(node.argument.literal)) edge(node.argument.literal, false)
      if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword && node.arguments[0] && ts.isStringLiteralLike(node.arguments[0])) edge(node.arguments[0], true, false)
      if (ts.isCallExpression(node) || ts.isNewExpression(node)) {
        const source = origin(node.expression)
        const binding = (ts.isPropertyAccessExpression(node.expression) && node.expression.name.text === 'bind') || (ts.isElementAccessExpression(node.expression) && ts.isStringLiteralLike(node.expression.argumentExpression) && node.expression.argumentExpression.text === 'bind')
        if (!binding && (source === 'global:fetch' || source === 'openapi-fetch') && from?.name !== 'api') add('server_access', file, source)
        if (!binding && source === 'global:EventSource' && from?.name !== 'stream') add('server_access', file, source)
        if (!binding && ts.isCallExpression(node) && from?.name !== 'api') {
          const method = calledName(node.expression)
          if (['setQueryData', 'setQueriesData', 'removeQueries', 'resetQueries'].includes(method)) add('cache_write', file, method)
          if (['setQueryData', 'getQueryData', 'getQueryState', 'ensureQueryData', 'fetchQuery', 'prefetchQuery'].includes(method) && node.arguments[0] && hasLocalArray(node.arguments[0])) add('query_key', file, method)
          if (method === 'writeSaved' && node.arguments[1] && hasLocalArray(node.arguments[1])) add('query_key', file, method)
        }
      }
      if (from?.name !== 'api' && ts.isPropertyAssignment(node) && node.name.getText(tree).replaceAll(/['"]/g, '') === 'queryKey' && hasLocalArray(node.initializer)) add('query_key', file, 'queryKey')
      if (from?.name !== 'api' && ts.isShorthandPropertyAssignment(node) && node.name.text === 'queryKey') {
        const symbol = checker.getShorthandAssignmentValueSymbol(node)
        const declaration = symbol?.valueDeclaration
        if (declaration && ts.isVariableDeclaration(declaration) && declaration.initializer && hasLocalArray(declaration.initializer)) add('query_key', file, 'queryKey')
      }
      ts.forEachChild(node, visit)
    }
    visit(tree)
  }

  const runtime = edges.filter(edge => edge.runtime)
  const adjacency = new Map<string, string[]>()
  for (const edge of runtime) adjacency.set(edge.from, [...(adjacency.get(edge.from) ?? []), edge.to])
  const reaches = (start: string, goal: string, visited = new Set<string>(), graph = adjacency): boolean => {
    if (start === goal) return true
    if (visited.has(start)) return false
    visited.add(start)
    return (graph.get(start) ?? []).some(next => reaches(next, goal, visited, graph))
  }
  // Startup must set the verified origin before the API client captures it.
  const staticGraph = new Map<string, string[]>()
  for (const edge of runtime.filter(edge => edge.static)) {
    staticGraph.set(edge.from, [...(staticGraph.get(edge.from) ?? []), edge.to])
  }
  // main.tsx is checked too: its static imports evaluate before any start-up step runs.
  for (const entry of ['main.tsx', ...owners.flatMap(item => item.entries).filter(entry => entry.endsWith('/startup.ts'))]) {
    if (reaches(entry, 'api/transport/index.ts', new Set(), staticGraph)) add('startup_api', entry, 'api/transport/index.ts')
  }
  for (const edge of runtime) {
    if (reaches(edge.to, edge.from)) add('runtime_cycle', edge.from, edge.to)
  }
  for (const file of sourceNames) {
    if ((owner(file)?.name === 'test-support' || /\.test\.tsx?$/.test(file)) && file !== 'main.tsx' && reaches('main.tsx', file)) add('production_test', 'main.tsx', file)
  }
  return findings.sort()
}

function compare(actual: string[], baseline: string[]): string[] {
  const problems: string[] = []
  if (new Set(baseline).size !== baseline.length) problems.push('DUPLICATE baseline entry')
  if (JSON.stringify([...baseline].sort()) !== JSON.stringify(baseline)) problems.push('UNSORTED baseline')
  for (const entry of actual) if (!baseline.includes(entry)) problems.push(`NEW ${entry}`)
  for (const entry of baseline) if (!actual.includes(entry)) problems.push(`STALE ${entry}`)
  return problems
}

describe('frontend architecture ratchet', () => {
  it('holds the canonical module graph with exact shrinking exceptions', () => {
    const actual = scan(walk(src), moduleTable(map.replaceAll('\r\n', '\n')))
    const baseline = readFileSync(path.join(src, 'architecture.baseline.txt'), 'utf8').trim().split('\n').filter(Boolean)
    expect(compare(actual, baseline), actual.join('\n')).toEqual([])
  }, 60_000)

  for (const fixture of fixtures) {
    it(fixture.name, () => {
      const owners = moduleTable(map.replaceAll('\r\n', '\n')).map(owner => ({ ...owner, contract: '-', entries: owner.entries.filter(entry => Object.keys(fixture.sources).some(file => matches(file, entry))), ...fixture.overrides?.[owner.name] }))
      const findings = scan(fixture.sources, owners)
      expect(findings.filter(line => line.startsWith(`${fixture.rule} ::`)), findings.join('\n')).toHaveLength(fixture.count ?? 1)
    })
  }

  it('rejects new, stale, duplicate and unsorted exceptions', () => {
    const a = 'dependency :: a.ts :: b.ts #1', b = 'dependency :: a.ts :: c.ts #1'
    expect(compare([a], [])).toEqual([`NEW ${a}`])
    expect(compare([], [a])).toEqual([`STALE ${a}`])
    expect(compare([a], [a, a])).toContain('DUPLICATE baseline entry')
    expect(compare([a, b], [b, a])).toContain('UNSORTED baseline')
    expect(compare([a, a.replace('#1', '#2')], [a])).toEqual([`NEW ${a.replace('#1', '#2')}`])
  })

  it('relocates a pure rename one-for-one rather than counting a fix', () => {
    const owners = moduleTable(map.replaceAll('\r\n', '\n')).map(owner => ({ ...owner, contract: '-', entries: [] }))
    const source = "import { value } from '../agents/private'"
    const target = { 'app/agents/private.ts': 'export const value = 1' }
    const before = scan({ ...target, 'app/design/old.ts': source }, owners)
    const reversed = Object.fromEntries(Object.entries({ ...target, 'app/design/old.ts': source }).reverse())
    expect(scan(reversed, owners)).toEqual(before)
    const after = scan({ ...target, 'app/design/renamed.ts': source }, owners)
    expect(before).toHaveLength(2)
    expect(after).toEqual(before.map(entry => entry.replace('app/design/old.ts', 'app/design/renamed.ts')))
    expect(compare(after, before)).toEqual([
      ...after.filter(entry => !before.includes(entry)).map(entry => `NEW ${entry}`),
      ...before.filter(entry => !after.includes(entry)).map(entry => `STALE ${entry}`),
    ])
    expect(compare(after, after)).toEqual([])
    const repeated = scan({ ...target, 'app/design/old.ts': `${source}; ${source}` }, owners)
    expect(compare(repeated, before).some(entry => entry.startsWith('NEW ') && entry.endsWith('#2'))).toBe(true)
  })
})
