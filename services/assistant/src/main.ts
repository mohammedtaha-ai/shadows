import {readFileSync,statSync} from 'node:fs';
import {isAbsolute} from 'node:path';
import {createHermesClient} from '@shadows/hermes-client';
import {Type} from '@sinclair/typebox';
import {Value} from '@sinclair/typebox/value';
import {openService} from './service.js';
import {safeErrors} from './protocol.js';
const Config=Type.Object({database:Type.String(),hermes:Type.Object({origin:Type.String(),network:Type.Optional(Type.String()),keyFile:Type.String(),timeoutMs:Type.Optional(Type.Integer({minimum:10,maximum:10000}))},{additionalProperties:false})},{additionalProperties:false});
const emit=(value:unknown)=>process.stdout.write(JSON.stringify(value)+'\n');
async function main(){
  const args=process.argv.slice(2);
  if(args.length!==2||args[0]!=='--config'||!isAbsolute(args[1]!))throw new Error('unavailable');
  if(statSync(args[1]!).size>16384)throw new Error('unavailable');
  const config:unknown=JSON.parse(readFileSync(args[1]!,'utf8').replace(/^\uFEFF/,''));
  if(!Value.Check(Config,config)||!isAbsolute(config.database)||!isAbsolute(config.hermes.keyFile)||statSync(config.hermes.keyFile).size>4096)throw new Error('unavailable');
  const key=readFileSync(config.hermes.keyFile,'utf8').trim();
  const service=openService({database:config.database,client:createHermesClient({...config.hermes,key})});
  emit({id:0,ok:true,result:{installationId:service.installationId}});
  try{
    let buffer=Buffer.alloc(0);
    for await(const chunk of process.stdin){
      buffer=Buffer.concat([buffer,Buffer.from(chunk)]);
      if(buffer.length>65536)throw new Error('request_invalid');
      let end:number;
      while((end=buffer.indexOf(10))>=0){
        const line=buffer.subarray(0,end);buffer=buffer.subarray(end+1);
        let id=0;
        try{
          const frame:unknown=JSON.parse(line.toString('utf8'));
          if(typeof frame!=='object'||frame===null||Array.isArray(frame)||Object.keys(frame).some(key=>!['id','request'].includes(key)))throw new Error('request_invalid');
          const envelope=frame as {id?:unknown;request?:unknown};
          if(!Number.isSafeInteger(envelope.id)||Number(envelope.id)<1)throw new Error('request_invalid');
          id=Number(envelope.id);
          emit({id,ok:true,result:await service.call(envelope.request)});
        }catch(error){const code=error instanceof Error&&safeErrors.has(error.message)?error.message:'unavailable';emit({id,ok:false,error:code});}
      }
    }
  }finally{service.close();}
}
main().catch(()=>{process.stderr.write('Local assistant process stopped.\n');process.exitCode=1;});
