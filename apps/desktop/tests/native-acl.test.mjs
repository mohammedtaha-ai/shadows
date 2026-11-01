import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,readdirSync} from 'node:fs';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

const inventoryMode=process.argv.includes('--wrapper-inventory');
const aclTest=inventoryMode?()=>{}:test;
const require=createRequire(import.meta.url);
let ts;
try { ts=require('../../../web/node_modules/typescript'); }
catch { ts=require('../node_modules/typescript'); }

// Parse calls rather than string occurrences so types, comments and test data cannot grant coverage.
const core='@tauri-apps/api/core';
const moduleKey=file=>path.normalize(file).replace(/\.tsx?$/,'');
const bridgeOf=ast=>/(^|[/\\])native\.ts$/.test(ast.fileName)&&ast.statements.find(n=>ts.isFunctionDeclaration(n)&&
 n.name?.text==='nativeCall'&&n.parameters[0]?.name.getText(ast)==='command');
// A caller used as a value could carry a command out of sight, so every use must be a direct call.
const nameOnly=node=>{
 const parent=node.parent;
 return (ts.isPropertyAccessExpression(parent)||ts.isPropertyAssignment(parent)||ts.isPropertyDeclaration(parent)||
  ts.isPropertySignature(parent)||ts.isMethodDeclaration(parent)||ts.isMethodSignature(parent))&&parent.name===node||
  ts.isImportSpecifier(parent)||ts.isImportClause(parent)||ts.isNamespaceImport(parent);
};
function wrapperCalls(sources) {
 const commands=new Map();
 const asts=Object.entries(sources).filter(([file])=>!/\.(test|spec)\.tsx?$/.test(file)).map(([file,text])=>{
  const ast=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,file.endsWith('.tsx')?ts.ScriptKind.TSX:ts.ScriptKind.TS);
  assert.equal(ast.parseDiagnostics.length,0,`${file}: TypeScript must parse`);
  return ast;
 });
 const bridges=new Set(asts.filter(bridgeOf).map(ast=>moduleKey(ast.fileName)));
 for(const ast of asts) {
  const file=ast.fileName;
  const native=module=>module===core||module.startsWith('.')&&bridges.has(moduleKey(path.join(path.dirname(file),module)));
  const callers=new Set(),namespaces=new Set();
  for(const statement of ast.statements) {
   if(!(ts.isImportDeclaration(statement)||ts.isExportDeclaration(statement))||!statement.moduleSpecifier||
      !ts.isStringLiteral(statement.moduleSpecifier))continue;
   const module=statement.moduleSpecifier.text;
   if(!native(module))continue;
   const bindings=ts.isImportDeclaration(statement)?statement.importClause?.namedBindings:statement.exportClause;
   if(ts.isExportDeclaration(statement)) {
    const names=bindings&&ts.isNamedExports(bindings)?bindings.elements.map(e=>(e.propertyName??e.name).text):['*'];
    assert.ok(!names.some(n=>n==='*'||n==='invoke'||n==='nativeCall'),`${file}: native callers must not be re-exported`);
    continue;
   }
   if(bindings&&ts.isNamedImports(bindings))for(const spec of bindings.elements) {
    const name=(spec.propertyName??spec.name).text;
    if(name===(module===core?'invoke':'nativeCall'))callers.add(spec.name.text);
   }
   if(module===core&&bindings&&ts.isNamespaceImport(bindings))namespaces.add(bindings.name.text);
  }
  const bridge=bridgeOf(ast);
  if(bridge)callers.add('nativeCall');
  const visit=node=>{
   const member=namespaces.has(node.text)&&ts.isPropertyAccessExpression(node.parent)&&node.parent.expression===node;
   if(ts.isIdentifier(node)&&!nameOnly(node)&&node!==bridge?.name&&(callers.has(node.text)||
      namespaces.has(node.text)&&(!member||node.parent.name.text==='invoke'))) {
    const call=member?node.parent.parent:node.parent;
    const direct=ts.isCallExpression(call)&&call.expression===(member?node.parent:node);
    assert.ok(direct,`${file}: native callers must be called directly`);
    const argument=call.arguments[0];
    if(argument&&(ts.isStringLiteral(argument)||ts.isNoSubstitutionTemplateLiteral(argument))) {
     const files=commands.get(argument.text)??new Set();files.add(file);commands.set(argument.text,files);
    } else {
     let enclosing=call.parent;
     while(enclosing&&!ts.isFunctionDeclaration(enclosing))enclosing=enclosing.parent;
     assert.ok(bridge&&enclosing===bridge&&argument&&ts.isIdentifier(argument)&&argument.text==='command',
      `${file}: native wrapper command must be a literal`);
    }
   }
   ts.forEachChild(node,visit);
  };
  visit(ast);
 }
 return commands;
}
function wrapperInventory() {
 const result={};
 for(const [surface,root] of [
  ['main',new URL('../../../web/src/desktop/',import.meta.url)],
  ['companion',new URL('../src/platform/',import.meta.url)],
 ]) {
  const sources={};
  const walk=directory=>{
   for(const entry of readdirSync(directory,{withFileTypes:true})) {
    const url=new URL(entry.name+(entry.isDirectory()?'/':''),directory);
    if(entry.isDirectory())walk(url);
    else if(/\.tsx?$/.test(entry.name))sources[fileURLToPath(url)]=readFileSync(url,'utf8');
   }
  };
  walk(root);
  for(const command of wrapperCalls(sources).keys())(result[command]??=[]).push(surface);
 }
 return result;
}
if(inventoryMode)process.stdout.write(JSON.stringify(wrapperInventory()));

const bridge=`import {invoke} from '@tauri-apps/api/core';
 export function nativeCall<T>(command:string,args?:Record<string,unknown>):Promise<T>{return invoke<T>(command,args)}`;
aclTest('wrapper inventory resolves imported aliases and namespace calls without counting comments or tests',()=>{
 const sources={
  'main.ts':`import {invoke as call} from '@tauri-apps/api/core';
   import {nativeCall as native} from './native';
   import * as core from '@tauri-apps/api/core';
   // call('comment');
   const text="call('string')";
   call<{ok:true}>('one');native('two');core.invoke('three');`,
  'native.ts':bridge,
  'nested/view.tsx':`import {nativeCall} from '../native';const View=()=><b>{String(nativeCall('four'))}</b>;`,
  'ignored.test.ts':`import {invoke} from '@tauri-apps/api/core';invoke('test_only');`,
 };
 assert.deepEqual([...wrapperCalls(sources).keys()].sort(),['four','one','three','two']);
 assert.deepEqual([...wrapperCalls({'main.ts':`function invoke(){};invoke('unrelated');`}).keys()],[]);
 assert.deepEqual([...wrapperCalls({'main.ts':`import * as core from '@tauri-apps/api/core';core.convertFileSrc('x');`}).keys()],[]);
});

aclTest('wrapper inventory refuses commands it cannot read from a literal direct call',()=>{
 for(const [text,reason] of [
  [`import {invoke} from '@tauri-apps/api/core';invoke(command);`,/must be a literal/],
  [`import {invoke} from '@tauri-apps/api/core';const COMMAND='one';invoke(COMMAND);`,/must be a literal/],
  [`import {invoke} from '@tauri-apps/api/core';invoke(\`x_\${name}\`);`,/must be a literal/],
  [`import {invoke} from '@tauri-apps/api/core';export const call=invoke;`,/called directly/],
  [`import {invoke} from '@tauri-apps/api/core';run(invoke,'one');`,/called directly/],
  [`import * as core from '@tauri-apps/api/core';const c=core;c.invoke('one');`,/called directly/],
  [`import {nativeCall} from './native';const call=nativeCall;call('one');`,/called directly/],
  [`export {invoke} from '@tauri-apps/api/core';`,/re-exported/],
  [`export * from './native';`,/re-exported/],
 ])assert.throws(()=>wrapperCalls({'native.ts':bridge,'main.ts':text}),new RegExp(`main\\.ts: native .*${reason.source}`));
});


aclTest('companion submission and editor commands have generated metadata and companion-only grants',()=>{
 const root=new URL('../src-tauri/gen/schemas/',import.meta.url);
 const manifests=JSON.parse(readFileSync(new URL('acl-manifests.json',root),'utf8'));
 const capabilities=JSON.parse(readFileSync(new URL('capabilities.json',root),'utf8'));
 const permissions=manifests['__app-acl__'].permissions;
 for(const name of ['companion_send','companion_input']) {
  const id=name.replaceAll('_','-');
  assert.deepEqual(permissions['allow-'+id]?.commands.allow,[name],`${name} must have generated allow metadata`);
  assert.deepEqual(permissions['deny-'+id]?.commands.deny,[name],`${name} must have generated deny metadata`);
  for(const label of ['main','companion']) {
   const commands=Object.values(capabilities).filter(c=>c.local&&c.windows.includes(label))
    .flatMap(c=>c.permissions.flatMap(p=>permissions[p]?.commands.allow??[]));
   assert.equal(commands.includes(name),label==='companion',`${name} must be granted only to companion`);
  }
 }
});

aclTest('generated native permissions grant sidecar access only to main',()=>{
 const root=new URL('../src-tauri/gen/schemas/',import.meta.url);
 const manifests=JSON.parse(readFileSync(new URL('acl-manifests.json',root),'utf8'));
 const capabilities=JSON.parse(readFileSync(new URL('capabilities.json',root),'utf8'));
 const permissions=manifests['__app-acl__'].permissions;
 const commands=label=>Object.values(capabilities).filter(c=>c.local&&c.windows.includes(label))
  .flatMap(c=>c.permissions.flatMap(p=>permissions[p]?.commands.allow??[]));
 assert.deepEqual([...commands('companion')].sort(),['companion_state','companion_ready','companion_failed','drag_companion','open_main_from_companion','companion_input','companion_send','companion_panel','answer_companion_permission','select_companion_agent'].sort());
 for(const name of ['companion_input','companion_send'])assert.ok(!commands('main').includes(name));
 assert.ok(commands('main').includes('aylul_call'),'main must reach the merged local sidecar');
 assert.ok(!commands('main').includes('assistant_call'),'retired assistant chat must have no main capability');
 for(const command of ['companion_binding','target_companion_thread','set_companion_previews','navigation_snapshot','ack_navigation','desktop_connection','desktop_connection_config','save_desktop_connection','retry_desktop_connection','open_external']) {
   assert.ok(commands('main').includes(command),'main must reach local assistant operations');
   assert.ok(!commands('companion').includes(command),'companion must not reach local assistant authority');
 }
 for(const command of ['assistant_call','assistant_local_call','assistant_sources'])assert.ok(!commands('main').includes(command)&&!commands('companion').includes(command),'retired chat commands have no capability');
 // The panel grid binds an agent through native, which owns the binding rules.
 assert.ok(commands('main').includes('select_companion_agent')&&commands('companion').includes('select_companion_agent'),'the panel selects through native');
 assert.ok(commands('main').includes('set_companion_visible'),'main keeps companion controls');
 assert.ok(!commands('companion').includes('aylul_call'),'companion must not approve agent requests');
 assert.ok(!commands('companion').includes('auth_action'),'companion must not receive account authority');
 assert.ok(permissions['allow-aylul-call']?.commands.allow.includes('aylul_call'),'command metadata must be generated');
});

 aclTest('appearance refresh is granted only to main',()=>{
 const root=new URL('../src-tauri/gen/schemas/',import.meta.url);
 const manifests=JSON.parse(readFileSync(new URL('acl-manifests.json',root),'utf8'));
 const capabilities=JSON.parse(readFileSync(new URL('capabilities.json',root),'utf8'));
 const permissions=manifests['__app-acl__'].permissions;
 const commands=label=>Object.values(capabilities).filter(c=>c.local&&c.windows.includes(label))
  .flatMap(c=>c.permissions.flatMap(p=>permissions[p]?.commands.allow??[]));
 assert.ok(commands('main').includes('refresh_companion_appearance'));
 assert.ok(!commands('companion').includes('refresh_companion_appearance'));
 assert.ok(permissions['allow-refresh-companion-appearance']?.commands.allow.includes('refresh_companion_appearance'));
 });

aclTest('desktop preferences are granted only to main',()=>{
 const root=new URL('../src-tauri/gen/schemas/',import.meta.url);
 const manifests=JSON.parse(readFileSync(new URL('acl-manifests.json',root),'utf8'));
 const capabilities=JSON.parse(readFileSync(new URL('capabilities.json',root),'utf8'));
 const permissions=manifests['__app-acl__'].permissions;
 for(const label of ['main','companion']) {
  const caps=Object.values(capabilities).filter(c=>c.local&&c.windows.includes(label));
  const commands=caps.flatMap(c=>c.permissions.flatMap(p=>permissions[p]?.commands.allow??[]));
  for(const name of ['desktop_preferences','set_desktop_preferences']) {
   assert.equal(commands.includes(name),label==='main');
   assert.ok(permissions['allow-'+name.replaceAll('_','-')]?.commands.allow.includes(name));
  }
  assert.ok(caps.every(c=>c.permissions.every(p=>!String(p).startsWith('autostart:'))));
 }
});
