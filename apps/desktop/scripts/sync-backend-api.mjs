import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,dirname} from 'node:path';
const source=resolve(process.argv[2]??'../../../shadows-backend/api/nylra-v1.d.ts');
const target=resolve('src/api/generated/nylra-v1.d.ts'),bytes=await readFile(source);
await mkdir(dirname(target),{recursive:true});await writeFile(target,bytes);
await writeFile(resolve('src/api/generated/provenance.json'),JSON.stringify({owner:'shadows-backend/api/nylra-v1.d.ts',sha256:createHash('sha256').update(bytes).digest('hex')},null,2)+'\n');
console.log('Backend generated types synchronized.');
