import {test} from 'node:test';
import assert from 'node:assert/strict';
import {existsSync} from 'node:fs';
test('portable_client_exports_without_the_backend_checkout', async () => {
  assert.ok(existsSync(new URL('../dist/index.js', import.meta.url)), 'Portable client build is missing');
  const client = await import('../dist/index.js');
  assert.equal(typeof client.createHermesClient, 'function');
  assert.equal(typeof client.readHermesTarget, 'function');
  assert.throws(() => client.createHermesClient({origin:'http://unreviewed.test',key:'synthetic-key'}), /hermes_config_invalid/);
});
