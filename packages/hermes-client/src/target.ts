export type HermesTarget={origin:string;timeoutMs:number};
/** Operator-owned origin. Neither request payloads nor provider settings choose it. */
export function readHermesTarget(env:NodeJS.ProcessEnv):HermesTarget{
 try{
  const raw=env.HERMES_API_URL;if(!raw||raw!==raw.trim())throw 0;
  const url=new URL(raw);
  if(url.username||url.password||url.search||url.hash||url.pathname!=='/')throw 0;
  const development=env.HERMES_NETWORK==='development-host'&&['127.0.0.1','localhost','[::1]','host.docker.internal'].includes(url.hostname);
  const privateService=env.HERMES_NETWORK==='private-service'&&url.hostname==='hermes';
  if(url.protocol!=='https:'&&!(url.protocol==='http:'&&(development||privateService)))throw 0;
  const value=env.HERMES_PROBE_TIMEOUT_MS??'3000';if(!/^\d+$/.test(value))throw 0;
  const timeoutMs=Number(value);if(timeoutMs<10||timeoutMs>10000)throw 0;
  return {origin:url.origin,timeoutMs};
 }catch{throw new Error('hermes_config_invalid');}
}
