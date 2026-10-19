import {defineConfig} from 'vite';
import {Agent} from 'node:https';
import {readFileSync} from 'node:fs';
export default defineConfig({server:{host:'127.0.0.1',port:4180,strictPort:true,proxy:{'/__assistant_backend':{
 target:'https://127.0.0.1:8443',changeOrigin:true,agent:new Agent({ca:readFileSync(process.env.NYLRA_DEV_CA!)}),
 rewrite:path=>path.replace('/__assistant_backend',''),
}}}});
