import {writeFileSync} from 'node:fs';
import {RequestSchema} from './protocol.js';
writeFileSync(new URL('../api/request.schema.json',import.meta.url),JSON.stringify(RequestSchema,null,2)+'\n');
