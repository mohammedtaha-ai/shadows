let nativeOrigin:string|undefined
/** Set once before importing any API consumer. Configuration changes reload main. */
export function setDaemonOrigin(origin:string):void {
  const url=new URL(origin)
  const octets=url.hostname.split('.').map(Number)
  const loopback=url.hostname==='[::1]' || (octets.length===4&&octets[0]===127&&octets.every(value=>Number.isInteger(value)&&value>=0&&value<=255))
  if(url.protocol!=='http:' || !loopback ||
    !url.port || url.username || url.password || url.pathname!=='/' || url.search || url.hash)
    throw new Error('core_origin_invalid')
  if(nativeOrigin!==undefined && nativeOrigin!==url.origin)throw new Error('core_configuration_changed')
  nativeOrigin=url.origin
}
export function daemonOrigin():string {
  return nativeOrigin ?? (import.meta.env.VITE_SHADOWS_URL ?? 'http://127.0.0.1:4318').replace(/\/+$/,'')
}
