import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, posix, resolve } from 'node:path';
import { API } from 'typescript/unstable/sync';
import * as ts from 'typescript/unstable/ast';

const desktop = resolve(import.meta.dirname, '../..');
const fixtures = join(import.meta.dirname, 'fixtures/boundaries');
type Sources = Record<string, string>;
type Edge = { target: string; detail: string; reexport: boolean };
const character = 'src/features/companion/';
const renderer = character + 'renderer/';
const animation = character + 'animation/';
const studio = character + 'studio/';
const isTest = (path: string) => /(?:^|\/)(?:tests|__tests__|fixtures|__fixtures__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/.test(path);
const isStudio = (path: string) => path.startsWith(studio) || path === 'src/studio.tsx';
const isCharacter = (path: string) => path.startsWith(renderer) || path.startsWith(animation);
const isEntry = (path: string) => path === renderer + 'index.ts' || path === animation + 'index.ts';

function files(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap(entry =>
    entry.isDirectory() ? files(join(dir, entry.name)) : [join(dir, entry.name)]);
}

function inspect(sources: Sources, exists = (path: string) => path in sources): string[] {
  const virtual = resolve(desktop, '__companion_boundaries__').replaceAll('\\', '/');
  const config = virtual + '/tsconfig.json';
  const content = new Map(Object.entries(sources).map(([path, text]) => [virtual + '/' + path, text]));
  content.set(config, JSON.stringify({ compilerOptions: { noResolve: true, noLib: true, jsx: 'react-jsx' }, files: Object.keys(sources) }));
  const api = new API({ fs: {
    readFile: path => content.get(path.replaceAll('\\', '/')),
    fileExists: path => content.has(path.replaceAll('\\', '/')) ? true : undefined,
    directoryExists: path => path.replaceAll('\\', '/').startsWith(virtual) ? true : undefined,
  } });
  const findings: string[] = [];
  const counts = new Map<string, number>();
  const graph = new Map<string, Edge[]>();
  function add(rule: string, path: string, detail: string) {
    const key = `${rule} :: ${path} :: ${detail}`;
    const count = (counts.get(key) ?? 0) + 1;
    counts.set(key, count);
    findings.push(`${key} #${count}`);
  }
  function target(path: string, specifier: string): string {
    if (!specifier.startsWith('.')) return specifier;
    const base = posix.normalize(posix.join(posix.dirname(path), specifier));
    const stem = base.replace(/\.[cm]?jsx?$/, '');
    const candidates = [base, ...['.ts', '.tsx', '.mts', '.cts', '.js', '.jsx', '.json'].map(ext => stem + ext),
      ...['index.ts', 'index.tsx', 'index.js', 'index.jsx'].map(name => base + '/' + name)];
    const found = candidates.find(exists);
    assert.ok(found, `Unresolved relative import: ${path} -> ${specifier}`);
    return found;
  }
  function visit(node: ts.Node, run: (node: ts.Node) => void) {
    run(node);
    node.forEachChild(child => { visit(child, run); });
  }
  const literal = (node: ts.Node | undefined): string | undefined =>
    node && (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) ? node.text : undefined;
  let snapshot: ReturnType<API['updateSnapshot']> | undefined;
  try {
    snapshot = api.updateSnapshot({ openProjects: [config] });
    const program = snapshot.getProjects()[0].program;
    for (const path of Object.keys(sources).sort()) {
      const tree = program.getSourceFile(virtual + '/' + path);
      assert.ok(tree, `Parser must return ${path}`);
      assert.equal(program.getSyntacticDiagnostics(virtual + '/' + path).length, 0, `Valid TypeScript required: ${path}`);
      const edges: Edge[] = [];
      graph.set(path, edges);
      visit(tree, node => {
        let specifier: ts.Node | undefined;
        if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) specifier = node.moduleSpecifier;
        if (ts.isImportTypeNode(node) && ts.isLiteralTypeNode(node.argument)) specifier = node.argument.literal;
        if (ts.isExternalModuleReference(node)) specifier = node.expression;
        if (ts.isCallExpression(node) && (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
            (ts.isIdentifier(node.expression) && node.expression.text === 'require'))) {
          specifier = node.arguments[0];
          if (literal(specifier) === undefined) add('dynamic-import', path, node.getText(tree).trim());
        }
        const name = literal(specifier);
        if (name !== undefined) {
          const resolved = target(path, name);
          edges.push({ target: resolved, detail: resolved, reexport: ts.isExportDeclaration(node) });
          if (/^@tauri-apps\//.test(resolved) && !path.startsWith('src/platform/')) add('tauri-import', path, resolved);
        }
        // Taking a reference also covers aliases such as `const request = fetch`.
        if (path.startsWith(renderer) && !isTest(path)) {
          const http = new Set(['fetch', 'XMLHttpRequest', 'EventSource']);
          if (ts.isIdentifier(node) && http.has(node.text)) {
            const parent = node.parent;
            if ((ts.isPropertyAccessExpression(parent) && parent.name === node) ||
                ((ts.isPropertyAssignment(parent) || ts.isMethodDeclaration(parent)) && parent.name === node)) return;
            add('renderer-http', path, node.text);
          }
          if (ts.isPropertyAccessExpression(node) && ['window', 'globalThis', 'self'].includes(node.expression.getText(tree)) && http.has(node.name.text)) {
            add('renderer-http', path, node.getText(tree));
          }
          if (ts.isElementAccessExpression(node) && ['window', 'globalThis', 'self'].includes(node.expression.getText(tree)) && http.has(literal(node.argumentExpression) ?? '')) {
            add('renderer-http', path, node.getText(tree));
          }
        }
      });
    }
    function reachable(start: string, forbidden: (path: string) => boolean, stop = (_path: string) => false, reexportsOnly = false): string[] {
      const seen = new Set<string>();
      const found = new Set<string>();
      function walk(path: string) {
        if (seen.has(path)) return;
        seen.add(path);
        if (forbidden(path)) found.add(path);
        if (!stop(path)) for (const edge of graph.get(path) ?? []) {
          if (!reexportsOnly || edge.reexport) walk(edge.target);
        }
      }
      walk(start);
      return [...found].sort();
    }
    const production = new Set(reachable('src/app/companion.tsx', () => true));
    for (const [path, edges] of graph) for (const edge of edges) {
      if (isCharacter(path) && !isTest(path)) {
        for (const forbidden of reachable(edge.target, p => p.startsWith('src/platform/') || isStudio(p))) {
          add('character-dependency', path, `${edge.detail} -> ${forbidden}`);
        }
      }
      if (production.has(path)) {
        if (isStudio(edge.target) || isTest(edge.target)) add('production-import', path, edge.detail);
      }
      if (isStudio(path) && !isTest(path)) {
        for (const forbidden of reachable(edge.target, p => isCharacter(p) && !isEntry(p), isCharacter, true)) {
          add('studio-entry', path, `${edge.detail} -> ${forbidden}`);
        }
      }
    }
    return findings.sort();
  } finally {
    snapshot?.dispose();
    api.close();
  }
}

function compare(actual: string[], baseline: string[]) {
  assert.equal(new Set(baseline).size, baseline.length, 'Baseline entries must be unique');
  return {
    new: actual.filter(entry => !baseline.includes(entry)),
    stale: baseline.filter(entry => !actual.includes(entry)),
  };
}

test('companion import boundaries match the exact shrinking baseline', () => {
  const sources = Object.fromEntries(files(join(desktop, 'src')).filter(path => /\.[cm]?[jt]sx?$/.test(path))
    .map(path => [posix.normalize(path.replaceAll('\\', '/').slice(desktop.length + 1)), readFileSync(path, 'utf8')]));
  const baseline = readFileSync(join(fixtures, 'boundaries-baseline.txt'), 'utf8').split(/\r?\n/).filter(Boolean);
  const actual = inspect(sources, path => path in sources || (existsSync(resolve(desktop, path)) && statSync(resolve(desktop, path)).isFile()));
  assert.deepEqual(compare(actual, baseline), { new: [], stale: [] }, 'Remove resolved exemptions; never add a violation');
});

for (const name of readdirSync(fixtures).filter(name => name.endsWith('.json')).sort()) {
  const fixture: { sources: Sources; expected: string[] } = JSON.parse(readFileSync(join(fixtures, name), 'utf8'));
  test(`boundary fixture: ${name}`, () => assert.deepEqual(inspect(fixture.sources), fixture.expected));
}

test('baseline rejects new, stale and duplicate entries', () => {
  assert.deepEqual(compare(['new'], ['stale']), { new: ['new'], stale: ['stale'] });
  assert.throws(() => compare([], ['same', 'same']), /Baseline entries must be unique/);
});
