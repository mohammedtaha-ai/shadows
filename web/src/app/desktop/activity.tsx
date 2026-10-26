import {isDesktop} from '@/desktop'
import {SessionsPanel} from './sessions-panel'
import {ApprovalsPanel} from './approvals-panel'
import {useActivity} from './use-activity'
export function Activity(){
  return isDesktop()?<LocalActivity/>:<p className="p-8">Local activity is available in the desktop app.</p>
}
function LocalActivity(){
  const {state,refresh}=useActivity()
  return <div className="space-y-8 overflow-y-auto p-8 [&_button]:rounded [&_button]:border [&_button]:px-3 [&_button]:py-2 [&_h2]:mb-3 [&_h2]:font-medium">
    <h1 className="text-lg font-medium">Local activity</h1>
    <SessionsPanel state={state} refresh={refresh}/><ApprovalsPanel state={state} refresh={refresh}/>
  </div>
}
