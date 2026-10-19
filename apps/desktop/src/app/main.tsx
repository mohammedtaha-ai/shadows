import React from 'react';
import { createRoot } from 'react-dom/client';
import './shell.css';
import './workspace.css';
import { WorkspaceShell, type WorkspacePage, type SidebarList } from './WorkspaceShell';
import { AccountPanel } from './AccountPanel';
import { hideToTray } from '../platform/windows';
import { authAction, cancelLogin, type AuthView, type AuthAction } from '../platform/auth';
import { CompanionControls } from './CompanionControls';
import { useAylulState } from '../data/aylul-state';
import { SessionsPanel } from '../features/sessions/SessionsPanel';
import { ApprovalsPanel } from '../features/approvals/ApprovalsPanel';
import { useAssistantConversation } from '../features/assistant/useAssistantConversation';
import { ConversationSidebar } from '../features/assistant/ConversationSidebar';
import { BotList } from '../features/assistant/BotList';
import { AssistantDetails } from '../features/assistant/AssistantDetails';
import { SettingsPanel } from './SettingsPanel';
import { ConversationView } from '../features/assistant/ConversationView';

const Studio = import.meta.env.DEV
  ? React.lazy(() => import('../features/companion/studio/CompanionStudio').then(module => ({default:module.CompanionStudio})))
  : null;
class PresenceBoundary extends React.Component<{children:React.ReactNode},{failed:boolean}> {
  state={failed:false};
  static getDerivedStateFromError(){return {failed:true};}
  render(){return this.state.failed?<p role="alert">AYLUL view is unavailable. Account and tray actions remain available.</p>:this.props.children;}
}

function Shell() {
  const aylul=useAylulState();
  const [studioOpen,setStudioOpen]=React.useState(false);
  const [page,setPage]=React.useState<WorkspacePage>('chat');
  const [list,setList]=React.useState<SidebarList>('chats');
  const [error,setError]=React.useState('');
  const [auth,setAuth]=React.useState<AuthView|null>(null);
  const [busy,setBusy]=React.useState(false);
  const [loggingIn,setLoggingIn]=React.useState(false);
  const [query,setQuery]=React.useState('');
  const conversation=useAssistantConversation(busy?null:auth);
  const [inspector,setInspector]=React.useState(()=>window.matchMedia('(min-width:951px)').matches);
  const once=React.useRef(false);
  async function act(action:AuthAction) {
    setBusy(true);setError('');setLoggingIn(action==='login'||action==='enroll');
    try {setAuth(await authAction(action));}
    catch(e) {setError(message(String(e))); if(action!=='status') {try {setAuth(await authAction('status'));} catch {setAuth(null);}}}
    finally {setBusy(false);setLoggingIn(false);}
  }
  React.useEffect(()=>{if(!once.current){once.current=true;void act('status');}},[]);
  return <>
    <WorkspaceShell page={page} onNavigate={setPage} list={list} onList={setList} auth={auth}
      chats={<ConversationSidebar controller={conversation} query={query} />} query={query} onQuery={setQuery}
      chat={<ConversationView controller={conversation} inspectorOpen={inspector} onInspector={()=>setInspector(open=>!open)} />}
      inspector={<AssistantDetails controller={conversation} onClose={()=>setInspector(false)} />} inspectorOpen={inspector}
      bots={<BotList />}
      activity={<><h1>Activity</h1><SessionsPanel state={aylul.state} refresh={aylul.refresh} /><ApprovalsPanel state={aylul.state} refresh={aylul.refresh} /></>}
      settings={<SettingsPanel
        account={<AccountPanel auth={auth} busy={busy} loggingIn={loggingIn} onAction={action=>void act(action)} onCancel={()=>void cancelLogin().catch(()=>setError('Unable to cancel sign-in.'))} />}
        studio={Studio && <section aria-labelledby="studio-dev"><h2 id="studio-dev">Development</h2>
          <button type="button" onClick={()=>setStudioOpen(value=>!value)}>{studioOpen?'Close development studio':'Open development studio'}</button>
          {studioOpen && <PresenceBoundary><React.Suspense fallback={<p>Loading development studio…</p>}><Studio /></React.Suspense></PresenceBoundary>}
        </section>}
        footer={<footer>Open from the tray or press Ctrl+Alt+Shift+F12.<br /><span lang="ar" dir="rtl">أيلول معك حين تحتاجه.</span></footer>} />}
      presence={<PresenceBoundary><CompanionControls /></PresenceBoundary>}
      onHide={()=>void hideToTray().catch(()=>setError('Unable to hide the panel. Please try again.'))} />
    {error && <p role="alert" className="workspace-error">{error}</p>}
  </>;
}
function message(code:string) {
  const messages:Record<string,string>={request_refused:'Sign-in was refused. Retry with your existing registration, or explicitly register a new device.',verification_refused:'Session verification was refused. Your device registration is preserved.',backend_unavailable:'Cannot reach the backend. Please retry.',cancelled:'Sign-in cancelled.',login_timeout:'Sign-in timed out. Start again.',storage_invalid:'Protected account storage could not be read. No identity was accepted.',storage_unavailable:'Unable to save protected account storage.',certificate_unavailable:'Development certificate is unavailable. Check the native launch configuration.'};
  return messages[code]??'Unable to complete this action. Please retry.';
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><Shell /></React.StrictMode>);
