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
 assert.ok(commands('main').includes('aylul_call'),'main must reach the merged local sidecar');
 assert.ok(commands('main').includes('set_companion_visible'),'main keeps companion controls');
 assert.ok(!commands('companion').includes('aylul_call'),'companion must not approve agent requests');
 assert.ok(!commands('companion').includes('auth_action'),'companion must not receive account authority');
 assert.ok(permissions['allow-aylul-call']?.commands.allow.includes('aylul_call'),'command metadata must be generated');
});
