import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

test('generated native permissions grant sidecar access only to main',()=>{
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

 test('appearance refresh is granted only to main',()=>{
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

test('desktop preferences are granted only to main',()=>{
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
