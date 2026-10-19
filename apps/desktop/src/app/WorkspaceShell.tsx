import React from 'react';
export type WorkspacePage='chat'|'activity'|'settings';
export type SidebarList='chats'|'bots';
type Props={page:WorkspacePage;onNavigate:(page:WorkspacePage)=>void;list:SidebarList;onList:(list:SidebarList)=>void;
  chat:React.ReactNode;bots:React.ReactNode;activity:React.ReactNode;settings:React.ReactNode;presence:React.ReactNode;onHide:()=>void};
const PAGES:[WorkspacePage,string][]=[['chat','Chat'],['activity','Activity'],['settings','Settings']];
export function WorkspaceShell({page,onNavigate,list,onList,chat,bots,activity,settings,presence,onHide}:Props){
  const [drawer,setDrawer]=React.useState(false);
  const go=(next:WorkspacePage)=>{setDrawer(false);onNavigate(next);};
  return <div className="workspace">
    <aside className="workspace-rail">
      <p className="eyebrow">NYLRA</p>
      <nav aria-label="Workspace">{PAGES.map(([id,label])=><button key={id} type="button" aria-current={page===id?'page':undefined} onClick={()=>go(id)}>{label}</button>)}</nav>
      <div className="workspace-presence" role="group" aria-label="AYLUL presence">{presence}</div>
      <button type="button" onClick={onHide}>Hide to tray</button>
    </aside>
    <div className="workspace-body">
      <div className="workspace-page" hidden={page!=='chat'}>
        <button type="button" className="workspace-drawer-button" aria-expanded={drawer} aria-controls="workspace-list" onClick={()=>setDrawer(open=>!open)} onKeyDown={e=>{if(e.key==='Escape')setDrawer(false);}}>{drawer?'Close list':'Chats and bots'}</button>
        <div className="workspace-chat">
          <div id="workspace-list" className="workspace-list" data-open={drawer} onKeyDown={e=>{if(e.key==='Escape')setDrawer(false);}}>
            <div role="group" aria-label="List">
              <button type="button" aria-pressed={list==='chats'} onClick={()=>onList('chats')}>Chats</button>
              <button type="button" aria-pressed={list==='bots'} disabled onClick={()=>onList('bots')}>Bots</button>
            </div>
            <div hidden={list!=='chats'}><p className="workspace-note">Switch conversations from the chat panel.</p></div>
            <div hidden={list!=='bots'}>{bots}</div>
          </div>
          <div className="workspace-main">{chat}</div>
        </div>
      </div>
      <div className="workspace-page workspace-scroll" hidden={page!=='activity'}>{activity}</div>
      <div className="workspace-page workspace-scroll" hidden={page!=='settings'}>{settings}</div>
    </div>
  </div>;
}
