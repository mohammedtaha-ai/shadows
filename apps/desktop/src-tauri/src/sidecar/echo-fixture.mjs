// Test fixture: answers `state` with its params, every other method with an error code.
import { createInterface } from 'node:readline';
createInterface({ input: process.stdin }).on('line', line => {
  const { id, method, params } = JSON.parse(line);
  process.stdout.write(JSON.stringify(method === 'state' ? { id, result: params } : { id, error: 'not_pending' }) + '\n');
});
