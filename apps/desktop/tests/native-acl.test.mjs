import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,readdirSync} from 'node:fs';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';

const inventoryMode=process.argv.includes('--wrapper-inventory');
const aclTest=inventoryMode?()=>{}:test;
const require=createRequire(import.meta.url);
let ts;
try { ts=require('../../../web/node_modules/typescript'); }
catch { ts=require('../node_modules/typescript'); }

// Parse calls rather than string occurrences so types, comments and test data cannot grant coverage.
function wrapperCalls(sources) {
 const commands=new Map();
 for(const [file,text] of Object.entries(sources)) {
  if(/\.(test|spec)\.tsx?$/.test(file))continue;
  const ast=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
  assert.equal(ast.parseDiagnostics.length,0,`${file}: TypeScript must parse`);
  const callers=new Set(),namespaces=new Set();
  for(const statement of ast.statements) {
   if(!ts.isImportDeclaration(statement)||!ts.isStringLiteral(statement.moduleSpecifier))continue;
   const module=statement.moduleSpecifier.text;
   const bindings=statement.importClause?.namedBindings;
   if(bindings&&ts.isNamedImports(bindings))for(const spec of bindings.elements) {
    const name=(spec.propertyName??spec.name).text;
    if((module==='@tauri-apps/api/core'&&name==='invoke')||
       (/^\.\/native(?:\.ts)?$/.test(module)&&name==='nativeCall'))callers.add(spec.name.text);
   }
   if(module==='@tauri-apps/api/core'&&bindings&&ts.isNamespaceImport(bindings))namespaces.add(bindings.name.text);
  }
  const bridge=/(^|[/\\])native\.ts$/.test(file)&&ast.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='nativeCall'&&
   n.parameters[0]?.name.getText(ast)==='command');
  if(bridge)callers.add('nativeCall');
  const visit=node=>{
   if(ts.isCallExpression(node)) {
    const callee=node.expression;
    const native=(ts.isIdentifier(callee)&&callers.has(callee.text))||
     (ts.isPropertyAccessExpression(callee)&&ts.isIdentifier(callee.expression)&&
      namespaces.has(callee.expression.text)&&callee.name.text==='invoke');
    if(native) {
     const argument=node.arguments[0];
     if(argument&&(ts.isStringLiteral(argument)||ts.isNoSubstitutionTemplateLiteral(argument))) {
      const files=commands.get(argument.text)??new Set();files.add(file);commands.set(argument.text,files);
     } else {
      let enclosing=node.parent;
      while(enclosing&&!ts.isFunctionDeclaration(enclosing))enclosing=enclosing.parent;
      assert.ok(bridge&&enclosing===bridge&&argument&&ts.isIdentifier(argument)&&argument.text==='command',
       `${file}: native wrapper command must be a literal`);
     }
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
    else if(entry.name.endsWith('.ts'))sources[fileURLToPath(url)]=readFileSync(url,'utf8');
   }
  };
  walk(root);
  for(const command of wrapperCalls(sources).keys())(result[command]??=[]).push(surface);
 }
 return result;
}
if(inventoryMode)process.stdout.write(JSON.stringify(wrapperInventory()));

aclTest('wrapper inventory resolves imported aliases and namespace calls without counting comments or tests',()=>{
 const sources={
  'main.ts':`import {invoke as call} from '@tauri-apps/api/core';
   import {nativeCall as native} from './native';
   import * as core from '@tauri-apps/api/core';
   // call('comment');
   const text="call('string')";
   call<{ok:true}>('one');native('two');core.invoke('three');`,
  'ignored.test.ts':`import {invoke} from '@tauri-apps/api/core';invoke('test_only');`,
 };
 assert.deepEqual([...wrapperCalls(sources).keys()].sort(),['one','three','two']);
 assert.deepEqual([...wrapperCalls({'main.ts':`function invoke(){};invoke('unrelated');`}).keys()],[]);
 assert.throws(()=>wrapperCalls({'main.ts':`import {invoke} from '@tauri-apps/api/core';invoke(command);`}),
  /main\.ts: native wrapper command must be a literal/);
});

aclTest('deny metadata never contributes a generated capability grant',()=>{
 const permissions={'deny-one':{commands:{deny:['one']}}};
 const capabilities={main:{local:true,windows:['main'],permissions:['deny-one']}};
 assert.deepEqual(grantedCommands(permissions,capabilities,'main'),[]);
});
function grantedCommands(permissions,capabilities,label) {
 return Object.values(capabilities).filter(c=>c.local&&c.windows.includes(label))
  .flatMap(c=>c.permissions.flatMap(p=>permissions[typeof p==='string'?p:p.identifier]?.commands.allow??[]));
}


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
 assert.deepEqual([...commands('companion')].sort(),['companion_state','companion_ready','companion_failed','drag_companion','open_main_from_companion','companion_input','companion_send'].sort());
 for(const name of ['companion_input','companion_send'])assert.ok(!commands('main').includes(name));
 assert.ok(commands('main').includes('aylul_call'),'main must reach the merged local sidecar');
 assert.ok(!commands('main').includes('assistant_call'),'retired assistant chat must have no main capability');
 for(const command of ['companion_binding','select_companion_agent','target_companion_thread','set_companion_previews','navigation_snapshot','ack_navigation','desktop_connection','desktop_connection_config','save_desktop_connection','retry_desktop_connection','open_external']) {
   assert.ok(commands('main').includes(command),'main must reach local assistant operations');
   assert.ok(!commands('companion').includes(command),'companion must not reach local assistant authority');
 }
 for(const command of ['assistant_call','assistant_local_call','assistant_sources'])assert.ok(!commands('main').includes(command)&&!commands('companion').includes(command),'retired chat commands have no capability');
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
