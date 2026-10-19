import React from 'react';
import {MarkIcon,ChatsIcon,BotIcon,SearchIcon,ActivityIcon,JobsIcon,SettingsIcon,PanelLeftIcon} from './icons';
import type {AuthView} from '../platform/auth';
export type WorkspacePage='chat'|'activity'|'settings';
export type SidebarList='chats'|'bots';
type Props={page:WorkspacePage;onNavigate:(page:WorkspacePage)=>void;list:SidebarList;onList:(list:SidebarList)=>void;auth:AuthView|null;
  chats:React.ReactNode;query:string;onQuery:(value:string)=>void;chat:React.ReactNode;inspector:React.ReactNode;inspectorOpen:boolean;bots:React.ReactNode;activity:React.ReactNode;settings:React.ReactNode;presence:React.ReactNode;onHide:()=>void};
export function WorkspaceShell({page,onNavigate,list,onList,auth,chats,query,onQuery,chat,inspector,inspectorOpen,bots,activity,settings,presence,onHide}:Props){
  const [drawer,setDrawer]=React.useState(false);
  const go=(next:WorkspacePage)=>{setDrawer(false);onNavigate(next);};
  const person=auth?.state==='signed_in'?auth.principal:null;
  const esc=(e:React.KeyboardEvent)=>{if(e.key==='Escape')setDrawer(false);};
  return <div className="workspace">
    <header className="workspace-top">
      <button type="button" className="workspace-drawer-button" aria-expanded={drawer} aria-controls="workspace-list" onClick={()=>setDrawer(open=>!open)} onKeyDown={esc}><PanelLeftIcon/>{drawer?'Close list':'Chats and bots'}</button>
      <span className="workspace-mark" aria-hidden="true"><MarkIcon/></span>
      <span className="workspace-wordmark">AYLUL</span>
      <span className="workspace-subtitle">/ personal workspace</span>
      <div className="workspace-presence" role="group" aria-label="AYLUL presence">{presence}
        <button type="button" onClick={onHide}>Hide to tray</button></div>
    </header>
    <div className="workspace-frame">
      <aside id="workspace-list" className="workspace-side" data-open={drawer} onKeyDown={esc}>
        <div className="workspace-tabs" role="group" aria-label="List">
          <button type="button" aria-pressed={page==='chat'&&list==='chats'} onClick={()=>{onList('chats');go('chat');}}><ChatsIcon/>Chats</button>
          <button type="button" aria-pressed={list==='bots'} onClick={()=>{onList('bots');go('chat');}}><BotIcon/>Bots</button>
        </div>
        <label className="workspace-search"><SearchIcon/><input type="search" placeholder="Search" aria-label="Search" value={query} disabled={!person||list!=='chats'} onChange={e=>onQuery(e.target.value)} /></label>
        <div className="workspace-listarea">
          <div hidden={list!=='chats'}>{chats}</div>
          <div hidden={list!=='bots'}>{bots}</div>
        </div>
        <nav aria-label="Workspace" className="workspace-bottom">
          <button type="button" aria-current={page==='activity'?'page':undefined} onClick={()=>go('activity')}><ActivityIcon/>Activity</button>
          <button type="button" disabled title="Not available yet"><JobsIcon/><span className="workspace-nav-text">Scheduled jobs<small>Not available yet</small></span></button>
          <button type="button" aria-current={page==='settings'?'page':undefined} onClick={()=>go('settings')}><SettingsIcon/>Settings</button>
        </nav>
        <div className="workspace-account">{person
          ?<><span className="workspace-initial" aria-hidden="true">{person.accountId.slice(0,1).toUpperCase()}</span><span className="workspace-who"><strong>{person.accountId}</strong><small>Personal workspace</small></span></>
          :<button type="button" className="workspace-signin" onClick={()=>go('settings')}>Sign in</button>}</div>
      </aside>
      <div className="workspace-body">
        <div className="workspace-page workspace-chat" hidden={page!=='chat'}><div className="workspace-main">{chat}</div><aside className="workspace-inspector" aria-label="Bot details" data-open={inspectorOpen}>{inspector}</aside></div>
        <div className="workspace-page workspace-scroll" hidden={page!=='activity'}>{activity}</div>
        <div className="workspace-page workspace-scroll" hidden={page!=='settings'}>{settings}</div>
      </div>
    </div>
  </div>;
}
