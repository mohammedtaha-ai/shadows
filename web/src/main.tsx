import './index.css'
import {bootApplication,isDesktop} from './desktop/startup'

const root=document.getElementById('root')
if(!root)throw new Error('index.html has no #root')
void bootApplication(isDesktop(),async ()=>{
  const desktop=await import('./desktop/startup')
  const {ConnectionSettings}=await import('./app/desktop/startup')
  await desktop.prepareDesktop(<ConnectionSettings/>)
  desktop.externalLinks()
},async ()=>{
  const {mountApplication}=await import('./app-root')
  mountApplication(root)
  if(isDesktop()){
    const {connectNavigation}=await import('./desktop')
    const {router}=await import('./router')
    await connectNavigation(async destination=>{
      switch(destination.kind){
        case 'companionSettings':await router.navigate({to:'/settings',hash:'companion-settings'});return
        case 'agentDraft':await router.navigate({to:'/aylul/agents/$slug/new',params:{slug:destination.agent}});return
        case 'conversation':await router.navigate({to:'/projects/$projectId/threads/$threadId',params:{projectId:destination.projectId,threadId:destination.threadId}});return
      }
    },async()=>{await router.load()})
  }
}).catch(()=>{root.textContent='Unable to start AYLUL. Reload to retry.'})
