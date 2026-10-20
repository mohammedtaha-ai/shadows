export interface ConnectionView {
  revision:number
  state:'unconfigured'|'starting'|'ready'|'failed'
  configurationId:string|null
  origin:string|null
  error:string|null
}
export interface ConnectionBridge {
  listen(callback:(view:ConnectionView)=>void):Promise<()=>void>
  snapshot():Promise<ConnectionView>
}
export function isDesktop():boolean {
  return typeof window!=='undefined' && '__TAURI_INTERNALS__' in window
}
export async function bootApplication(
  desktop:boolean,prepare:()=>Promise<void>,mount:()=>Promise<void>,
):Promise<void> {
  if(desktop) await prepare()
  await mount()
}
export async function waitForReady(
  bridge:ConnectionBridge,onState:(view:ConnectionView)=>void,
):Promise<ConnectionView> {
  return new Promise((resolve,reject)=>{
    let revision=-1,disposed=false,off:(()=>void)|undefined
    const accept=(view:ConnectionView)=>{
      if(disposed || view.revision<=revision)return
      revision=view.revision
      onState(view)
      if(view.state==='ready' && view.origin!==null){
        disposed=true
        off?.()
        resolve(view)
      }
    }
    void bridge.listen(accept).then(dispose=>{
      if(disposed){dispose();return}
      off=dispose
      return bridge.snapshot().then(accept)
    }).catch(error=>{disposed=true;off?.();reject(error)})
  })
}
