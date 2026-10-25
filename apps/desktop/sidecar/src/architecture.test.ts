import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { API } from 'typescript/unstable/sync';
import * as ts from 'typescript/unstable/ast';

const root = dirname(fileURLToPath(import.meta.url));
type Sources = Record<string, string>;
type Violation = { rule: string; detail: string };
const read = (path: string) => readFileSync(path, 'utf8');
function files(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory() ? files(join(dir, e.name)) : [join(dir, e.name)]);
}

/** Parse text through the installed TypeScript 7 API without evaluating any inspected module. */
function parse(sources: Sources, run: (trees: Map<string, ts.SourceFile>) => void) {
  const virtual = resolve(root, '__architecture__').replaceAll('\\', '/');
  const config = virtual + '/tsconfig.json';
  const content = new Map(Object.entries(sources).map(([name, text]) => [virtual + '/' + name, text]));
  content.set(config, JSON.stringify({ compilerOptions: { noResolve: true, noLib: true }, files: Object.keys(sources) }));
  const api = new API({ fs: {
    readFile: path => content.get(path.replaceAll('\\', '/')),
    fileExists: path => content.has(path.replaceAll('\\', '/')) ? true : undefined,
    directoryExists: path => path.replaceAll('\\', '/').startsWith(virtual) ? true : undefined,
  } });
  try {
    const snapshot = api.updateSnapshot({ openProjects: [config] });
    const program = snapshot.getProjects()[0].program;
    const trees = new Map<string, ts.SourceFile>();
    for (const name of Object.keys(sources)) {
      const tree = program.getSourceFile(virtual + '/' + name);
      assert.ok(tree, `Parser must return ${name}`);
      assert.equal(program.getSyntacticDiagnostics(virtual + '/' + name).length, 0, `Valid TypeScript required: ${name}`);
      trees.set(name, tree);
    }
    run(trees);
    snapshot.dispose();
  } finally { api.close(); }
}
function walk(node: ts.Node, visit: (n: ts.Node, parent?: ts.Node) => void, parent?: ts.Node) {
  visit(node, parent);
  node.forEachChild(child => { walk(child, visit, node); });
}
function literal(node: ts.Node | undefined, constants: Map<string, string>): string | undefined {
  if (!node) return undefined;
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  if (ts.isIdentifier(node)) return constants.get(node.text);
  if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken) {
    const a = literal(node.left, constants), b = literal(node.right, constants);
    if (a !== undefined && b !== undefined) return a + b;
  }
  return undefined;
}
function member(node: ts.Node): string | undefined {
  if (ts.isPropertyAccessExpression(node)) return node.name.text;
  if (ts.isElementAccessExpression(node)) return literal(node.argumentExpression, new Map());
  return undefined;
}
function inspect(path: string, tree: ts.SourceFile): Violation[] {
  const violations: Violation[] = [];
  const add = (rule: string, node: ts.Node) => violations.push({ rule, detail: `${path}:${node.pos}:${node.getText(tree).trim()}` });
  const constructors = new Set(['Sessions', 'Approvals', 'AgentSettings', 'SessionsStore', 'ApprovalsStore', 'SettingsStore']);
  const constants = new Map<string, string>();
  const aliases = new Set(constructors);
  const connections = new Set(['DatabaseSync']);
  walk(tree, node => {
    if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
      const bindings = node.importClause?.namedBindings;
      if (bindings && ts.isNamedImports(bindings)) for (const element of bindings.elements) {
        const original = (element.propertyName ?? element.name).text;
        if (constructors.has(original)) aliases.add(element.name.text);
        if (original === 'DatabaseSync') connections.add(element.name.text);
      }
    }
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      // Only a const keeps its text; a reassignable binding is dynamic SQL.
      const constant = ts.isVariableDeclarationList(node.parent) && (node.parent.flags & ts.NodeFlags.Const) !== 0;
      const value = literal(node.initializer, constants);
      if (value !== undefined && constant) constants.set(node.name.text, value);
      if (aliases.has(node.initializer.getText(tree)) || constructors.has(member(node.initializer) ?? '')) aliases.add(node.name.text);
      if (connections.has(node.initializer.getText(tree)) || member(node.initializer) === 'DatabaseSync') connections.add(node.name.text);
    }
  });
  const domain = path.split('/')[0];
  const tables: Record<string, string[]> = { sessions: ['sessions', 'steps'], approvals: ['decisions'], agents: ['rules', 'grant_uses'] };
  const isStore = path === `${domain}/store.ts` && domain in tables;
  const composition = path === 'app.ts';
  const foreign = Object.entries(tables).filter(([owner]) => owner !== domain).flatMap(([, names]) => names);
  walk(tree, (node, parent) => {
    let imported: string | undefined;
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) imported = literal(node.moduleSpecifier, constants);
    if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
      imported = literal(node.arguments[0], constants);
      if (imported === undefined) add('dynamic-import', node);
    }
    if (imported && /\/store(?:\.ts)?$/.test(imported) && !composition) {
      const owner = posix.normalize(posix.join(posix.dirname(path), imported)).split('/')[0];
      // A re-export would hand the private store to whoever imports the re-exporting module.
      if (owner !== domain || !(domain in tables) || ts.isExportDeclaration(node)) add('store-access', node);
    }
    const sqliteText = ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node) ? node.text : undefined;
    if (sqliteText === 'node:sqlite' && !isStore && path !== 'store/store.ts') add('connection', node);
    const bound = ts.isBindingElement(node) ? node.propertyName ?? node.name : undefined;
    if (bound && ['prepare', 'exec'].includes(bound.getText(tree))) add('sql-indirection', node);
    if (ts.isNewExpression(node)) {
      const name = node.expression.getText(tree);
      const prop = member(node.expression);
      if (!composition && (aliases.has(name) || (prop && constructors.has(prop)))) add('construction', node);
      if (path !== 'store/store.ts' && (connections.has(name) || prop === 'DatabaseSync')) add('connection', node);
    }
    if (['prepare', 'exec'].includes(member(node) ?? '') &&
        !(ts.isPropertyAccessExpression(node) && node.expression.kind === ts.SyntaxKind.RegularExpressionLiteral) &&
        !(parent && ts.isCallExpression(parent) && parent.expression.pos === node.pos && parent.expression.end === node.end)) {
      add('sql-indirection', node);
    }
    if (ts.isCallExpression(node) && ['prepare', 'exec'].includes(member(node.expression) ?? '')) {
      if (ts.isPropertyAccessExpression(node.expression) && node.expression.expression.kind === ts.SyntaxKind.RegularExpressionLiteral) return;
      if (!isStore && path !== 'store/store.ts') { add('sql-location', node); return; }
      const sql = literal(node.arguments[0], constants);
      if (sql === undefined) { add('dynamic-sql', node); return; }
      if (path === 'store/store.ts' && (member(node.expression) !== 'exec' || /\b(?:SELECT|INSERT|UPDATE|DELETE)\b/i.test(sql))) add('sql-location', node);
      if (isStore) {
        const names = [...sql.matchAll(/\b(?:FROM|JOIN|INTO|UPDATE|TABLE(?:\s+IF\s+NOT\s+EXISTS)?)\s+["'`\[]?([a-z_]+)/gi)].map(m => m[1].toLowerCase());
        // The word scan also catches a table named after a comma or inside an expression.
        if (names.filter(name => name !== 'set').some(name => !tables[domain].includes(name)) ||
            foreign.some(name => new RegExp(`\\b${name}\\b`, 'i').test(sql))) add('sql-owner', node);
      }
    }
  });
  return violations;
}
function handlerMethods(tree: ts.SourceFile): string[] {
  // Only the object makeHandlers returns is the method table; a handler's own return value is not.
  const names: string[] = [];
  walk(tree, node => {
    if (!ts.isFunctionDeclaration(node) || node.name?.text !== 'makeHandlers' || !node.body) return;
    for (const statement of node.body.statements) {
      if (!ts.isReturnStatement(statement) || !statement.expression || !ts.isObjectLiteralExpression(statement.expression)) continue;
      for (const property of statement.expression.properties) {
        if ('name' in property && property.name && (ts.isIdentifier(property.name) || ts.isStringLiteral(property.name))) names.push(property.name.text);
        else names.push(`unreadable:${property.getText(tree)}`);
      }
    }
  });
  return names.sort();
}
function wrapperMethods(tree: ts.SourceFile): string[] {
  const names = new Set<string>();
  walk(tree, node => {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'call') {
      const name = literal(node.arguments[0], new Map());
      if (name) names.add(name);
    }
  });
  return [...names].sort();
}
function parity(methods: string[], native: string[], wrappers: string[]) {
  const out: string[] = [];
  for (const method of new Set(native)) if (native.indexOf(method) !== native.lastIndexOf(method)) out.push(`rpc-native-duplicate:${method}`);
  for (const [label, inventory] of [['native', native], ['wrapper', wrappers]] as const) {
    for (const method of methods) if (!inventory.includes(method)) out.push(`rpc-${label}-missing:${method}`);
    for (const method of inventory) if (!methods.includes(method)) out.push(`rpc-${label}-extra:${method}`);
  }
  return out.sort();
}

test('sidecar ownership and RPC parity match the exact shrinking baseline', () => {
  const sources: Sources = {};
  for (const file of files(root).filter(f => f.endsWith('.ts') && !f.endsWith('.test.ts'))) {
    sources[file.slice(root.length + 1).replaceAll('\\', '/')] = read(file);
  }
  // These external owners are text inputs only; importing them would cross runtime boundaries.
  const wrappers = resolve(root, '../../../../web/src/desktop');
  const wrapperFiles = files(wrappers).filter(f => f.endsWith('.ts') && !f.endsWith('.test.ts'));
  for (const file of wrapperFiles) sources['external/' + file.split(/[\\/]/).at(-1)] = read(file);
  const nativeText = read(resolve(root, '../../src-tauri/src/sidecar/mod.rs'));
  const allowlist = nativeText.match(/pub const METHODS:[^=]+?= &\[([\s\S]*?)\];/);
  assert.ok(allowlist, 'The native method allowlist must exist');
  const native = [...allowlist[1].matchAll(/"([^"]+)"/g)].map(m => m[1]).sort();
  parse(sources, trees => {
    const violations: string[] = [];
    for (const [path, tree] of trees) if (!path.startsWith('external/')) {
      violations.push(...inspect(path, tree).map(v => `${v.rule}:${v.detail}`));
    }
    const wrappers = [...new Set([...trees].filter(([p]) => p.startsWith('external/')).flatMap(([, tree]) => wrapperMethods(tree)))].sort();
    violations.push(...parity(handlerMethods(trees.get('handlers.ts')!), native, wrappers));
    const baseline = read(join(root, 'architecture.baseline.txt')).trim().split('\n').filter(Boolean).sort();
    assert.deepEqual(violations.sort(), baseline, 'New violations and stale exemptions must fail');
  });
});

test('adversarial TypeScript fixtures reject aliases, namespaces, store access and foreign SQL', () => {
  const fixtures = JSON.parse(read(join(root, 'architecture.fixtures.json'))) as Array<{ path: string; source: string; rule: string | null }>;
  parse(Object.fromEntries(fixtures.map((f, i) => [`fixture${i}.ts`, f.source])), trees => {
    const missed = fixtures.flatMap((fixture, i) => {
      const violations = inspect(fixture.path, trees.get(`fixture${i}.ts`)!);
      // A null rule is a comment or unrelated string, which is no executable violation.
      const caught = fixture.rule ? violations.some(v => v.rule === fixture.rule) : violations.length === 0;
      return caught ? [] : [`${fixture.rule ?? 'none'} at ${fixture.path}: ${fixture.source}`];
    });
    assert.deepEqual(missed, [], 'Every fixture must be judged as declared');
  });
  assert.deepEqual(parity(['state'], ['other'], []), ['rpc-native-extra:other', 'rpc-native-missing:state', 'rpc-wrapper-missing:state']);
  assert.deepEqual(parity(['state'], ['state', 'state'], ['state']), ['rpc-native-duplicate:state']);
  parse({ 'handlers.ts': "export function makeHandlers() { return { state: () => { return { ok: 1 }; }, ...extra }; }" }, trees => {
    assert.deepEqual(handlerMethods(trees.get('handlers.ts')!), ['state', 'unreadable:...extra']);
  });
});
