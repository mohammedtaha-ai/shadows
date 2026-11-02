import {capabilities} from '@/desktop'
import {SessionsPanel} from './sessions-panel'
import {ApprovalsPanel} from './approvals-panel'
import {useActivity} from './use-activity'
export function Activity(){
  return capabilities.localCore?<LocalActivity/>:<div className="flex flex-1 items-center justify-center p-8"><p className="surface-panel rounded-2xl px-6 py-4 text-sm text-muted-foreground">Local activity is available in the desktop app.</p></div>
}
function LocalActivity(){
  const {state,refresh}=useActivity()
  return <div className="space-y-8 overflow-y-auto p-8 [&_button]:rounded-lg [&_button]:border [&_button]:border-accent-line/25 [&_button]:bg-accent-line/10 [&_button]:px-3 [&_button]:py-2 [&_h2]:mb-3 [&_h2]:font-semibold">
    <h1 className="text-lg font-semibold tracking-tight text-foreground">Local activity</h1>
    <SessionsPanel state={state} refresh={refresh}/><ApprovalsPanel state={state} refresh={refresh}/>
  </div>
}
