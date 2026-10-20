import {createRoot} from 'react-dom/client'
import {ConnectionSettings} from '../app/desktop/connection-settings'
import {setDaemonOrigin} from '../api/daemon-origin'
import {connectionBridge,openExternal} from './native'
import {waitForReady,type ConnectionView} from './runtime'

export async function prepareDesktop():Promise<void>{
  const element=document.getElementById('root')
  if(!element)throw new Error('missing_root')
  const root=createRoot(element)
  const render=(view:ConnectionView)=>root.render(<main className="mx-auto max-w-2xl p-8">
    <h1 className="mb-4 text-2xl">AYLUL</h1>
    <p role="status" className="mb-4">{view.state==='starting'?'Starting local Core…':view.error??'Configure local Core to continue.'}</p>
    <ConnectionSettings/>
  </main>)
  try{
    const ready=await waitForReady(connectionBridge,render)
    if(!ready.origin)throw new Error('core_unavailable')
    setDaemonOrigin(ready.origin)
    await watchConnection(ready.configurationId)
  }finally{root.unmount()}
}
export async function watchConnection(configurationId:string|null):Promise<()=>void>{
  let revision=-1
  const accept=(view:ConnectionView)=>{
    if(view.revision<=revision)return
    revision=view.revision
    if(view.configurationId!==configurationId){
      window.location.hash='/?settings=core';window.location.reload()
    }
  }
  const off=await connectionBridge.listen(accept)
  try{accept(await connectionBridge.snapshot())}catch(error){off();throw error}
  return off
}
export function externalLinks():void{
  document.addEventListener('click',event=>{
    const target=event.target instanceof Element?event.target.closest('a'):null
    if(!target?.href)return
    const url=new URL(target.href)
    if(['http:','https:'].includes(url.protocol)&&url.origin!==location.origin){
      event.preventDefault()
      void openExternal(url.href).catch(()=>{})
    }
  })
}
