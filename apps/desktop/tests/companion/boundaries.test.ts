import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, realpathSync, statSync } from 'node:fs';
import { join, posix, resolve } from 'node:path';
import { API } from 'typescript/unstable/sync';
import * as ts from 'typescript/unstable/ast';

const desktop = resolve(import.meta.dirname, '../..');
const fixtures = join(import.meta.dirname, 'fixtures/boundaries');
type Sources = Record<string, string>;
type Edge = { target: string; detail: string; reexport: boolean; local: boolean };
type Fixture = { sources: Sources; files?: string[]; expected: string[] };
const character = 'src/features/companion/';
const renderer = character + 'renderer/';
const animation = character + 'animation/';
const studio = character + 'studio/';
const isTest = (path: string) => /(?:^|\/)(?:tests|__tests__|fixtures|__fixtures__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/.test(path);
const isStudio = (path: string) => path.startsWith(studio) || path === 'src/studio.tsx';
const isCharacter = (path: string) => path.startsWith(renderer) || path.startsWith(animation);
const isEntry = (path: string) => path === renderer + 'index.ts' || path === animation + 'index.ts';
const isLocal = (specifier: string) => specifier.startsWith('.') || specifier.startsWith('/');
const isCode = (path: string) => /\.[cm]?[jt]sx?$/.test(path);
const globals = ['window', 'globalThis', 'self'];

function files(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap(entry =>
    entry.isDirectory() ? files(join(dir, entry.name)) : [join(dir, entry.name)]);
}

// Exact case keeps a mis-cased import, which Windows would resolve, from escaping the owner prefixes.
function onDisk(path: string): boolean {
  const full = resolve(desktop, path);
  return existsSync(full) && statSync(full).isFile() && realpathSync.native(full) === full;
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
    if (!isLocal(specifier)) return specifier;
    // Vite resolves a leading slash from the project root.
    const base = specifier.startsWith('/') ? posix.normalize(specifier.slice(1))
      : posix.normalize(posix.join(posix.dirname(path), specifier));
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
        // Vite bundles what these name, and a glob pattern cannot be reviewed as one edge.
        if (ts.isCallExpression(node) && node.expression.getText(tree) === 'import.meta.glob') {
          add('dynamic-import', path, node.getText(tree).trim());
        }
        if (ts.isNewExpression(node) && node.expression.getText(tree) === 'URL' &&
            node.arguments?.length === 2 && node.arguments[1].getText(tree) === 'import.meta.url') {
          specifier = node.arguments[0];
          if (literal(specifier) === undefined) add('dynamic-import', path, node.getText(tree).trim());
        }
        const name = literal(specifier);
        if (name !== undefined) {
          const resolved = target(path, name);
          edges.push({ target: resolved, detail: resolved, reexport: ts.isExportDeclaration(node), local: isLocal(name) });
          if (/^@tauri-apps\//.test(resolved) && !path.startsWith('src/platform/')) add('tauri-import', path, resolved);
        }
        // The injected Tauri globals reach the same IPC as the package, without an import.
        const member = ts.isIdentifier(node) ? node.text
          : ts.isElementAccessExpression(node) ? literal(node.argumentExpression) : undefined;
        if (member?.startsWith('__TAURI') && !path.startsWith('src/platform/')) add('tauri-import', path, member);
        // Taking a reference also covers aliases such as `const request = fetch`.
        if (path.startsWith(renderer) && !isTest(path)) {
          const http = new Set(['fetch', 'XMLHttpRequest', 'EventSource']);
          if (ts.isIdentifier(node) && http.has(node.text)) {
            const parent = node.parent;
            if ((ts.isPropertyAccessExpression(parent) && parent.name === node) ||
                ((ts.isPropertyAssignment(parent) || ts.isMethodDeclaration(parent)) && parent.name === node)) return;
            add('renderer-http', path, node.text);
          }
          // Any receiver counts: `(window).fetch` and `self.globalThis.fetch` reach the same global.
          if (ts.isPropertyAccessExpression(node) && http.has(node.name.text)) {
            add('renderer-http', path, node.getText(tree));
          }
          if (ts.isElementAccessExpression(node)) {
            const key = literal(node.argumentExpression);
            let receiver: ts.Node = node.expression;
            while (ts.isParenthesizedExpression(receiver)) receiver = receiver.expression;
            const owner = ts.isIdentifier(receiver) ? receiver.text
              : ts.isPropertyAccessExpression(receiver) ? receiver.name.text : '';
            if (key === undefined ? globals.includes(owner) : http.has(key)) add('renderer-http', path, node.getText(tree));
          }
        }
      });
    }
    function reachable(start: string, forbidden: (path: string) => boolean): string[] {
      const seen = new Set<string>();
      const found = new Set<string>();
      function walk(path: string) {
        if (seen.has(path)) return;
        seen.add(path);
        if (forbidden(path)) found.add(path);
        for (const edge of graph.get(path) ?? []) walk(edge.target);
      }
      walk(start);
      return [...found].sort();
    }
    const production = new Set(reachable('src/app/companion.tsx', () => true));
    // Studio may use feature modules production already ships, which own their own imports;
    // a module only studio reaches is studio code, so all of its imports are followed.
    function studioReach(start: string): string[] {
      const seen = new Set<string>();
      const found = new Set<string>();
      function walk(path: string) {
        if (seen.has(path)) return;
        seen.add(path);
        if (isCharacter(path) && !isEntry(path)) found.add(path);
        if (isCharacter(path) || isStudio(path)) return;
        for (const edge of graph.get(path) ?? []) if (edge.reexport || !production.has(path)) walk(edge.target);
      }
      walk(start);
      return [...found].sort();
    }
    for (const [path, edges] of graph) for (const edge of edges) {
      // A local code file outside the scanned tree would hide everything it imports.
      if (edge.local && isCode(edge.target) && !graph.has(edge.target)) add('outside-source', path, edge.detail);
      if (isCharacter(path) && !isTest(path)) {
        for (const forbidden of reachable(edge.target, p => p.startsWith('src/platform/') || isStudio(p))) {
          add('character-dependency', path, `${edge.detail} -> ${forbidden}`);
        }
      }
      if (production.has(path)) {
        if (isStudio(edge.target) || isTest(edge.target)) add('production-import', path, edge.detail);
      }
      if (isStudio(path) && !isTest(path)) {
        for (const forbidden of studioReach(edge.target)) {
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

const sources = Object.fromEntries(files(join(desktop, 'src')).filter(isCode)
  .map(path => [posix.normalize(path.replaceAll('\\', '/').slice(desktop.length + 1)), readFileSync(path, 'utf8')]));
const baseline = readFileSync(join(fixtures, 'boundaries-baseline.txt'), 'utf8').split(/\r?\n/).filter(Boolean);
const actual = inspect(sources, path => path in sources || onDisk(path));

test('companion import boundaries match the exact shrinking baseline', () => {
  assert.deepEqual(compare(actual, baseline), { new: [], stale: [] }, 'Remove resolved exemptions; never add a violation');
});

for (const name of readdirSync(fixtures).filter(name => name.endsWith('.json')).sort()) {
  const fixture: Fixture = JSON.parse(readFileSync(join(fixtures, name), 'utf8'));
  const exists = (path: string) => path in fixture.sources || (fixture.files ?? []).includes(path);
  test(`boundary fixture: ${name}`, () => assert.deepEqual(inspect(fixture.sources, exists), fixture.expected));
}

test('baseline rejects new, stale and duplicate entries', () => {
  assert.ok(baseline.length > 0);
  assert.deepEqual(compare(actual, baseline.slice(1)), { new: [baseline[0]], stale: [] });
  assert.deepEqual(compare(actual, [...baseline, 'studio-entry :: gone #1']), { new: [], stale: ['studio-entry :: gone #1'] });
  assert.throws(() => compare(actual, [...baseline, baseline[0]]), /Baseline entries must be unique/);
});

test('a source file resolves only with its exact case', () => {
  assert.equal(onDisk('src/app/companion.tsx'), true);
  assert.equal(onDisk('src/App/companion.tsx'), false);
  assert.equal(onDisk('src/app'), false);
});
