import './index.css'
import {bootApplication,isDesktop} from './desktop/runtime'

const root=document.getElementById('root')
if(!root)throw new Error('index.html has no #root')
void bootApplication(isDesktop(),async ()=>{
  const desktop=await import('./desktop/bootstrap')
  await desktop.prepareDesktop()
  desktop.externalLinks()
},async ()=>{
  const {mountApplication}=await import('./app-root')
  mountApplication(root)
}).catch(()=>{root.textContent='Unable to start AYLUL. Reload to retry.'})
