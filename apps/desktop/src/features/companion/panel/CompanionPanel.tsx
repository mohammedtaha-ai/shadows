import type {CompanionState,MainTarget} from '../../../platform/companion';
import {AgentPreview,AgentTile} from './AgentTile';
import './panel.css';

export function CompanionPanel({panel,status,onClose,onNavigate,onChat,onSelect,selecting,sound,soundPending,onSound,error}:{
  panel:CompanionState['panel'];status:string;onClose:()=>void;onNavigate:(target:MainTarget)=>void;
  onChat:()=>void;onSelect:(slug:string)=>void;selecting:boolean;sound:boolean;soundPending:boolean;
  onSound:()=>void;error:string;
}){
  const bound=panel.agents.find(agent=>agent.bound);
  return <section className="aylul-panel" aria-label="AYLUL panel">
    <header className="aylul-panel-head">
      <strong>AYLUL</strong>
      <span className="aylul-panel-pill" aria-label="Waiting requests">Ready</span>
      <button type="button" className="aylul-panel-close" aria-label="Close panel" onClick={onClose}>×</button>
    </header>
    <nav className="aylul-panel-topbar" aria-label="Panel navigation">
      <button type="button" aria-current="page">Home</button>
      <button type="button" onClick={onChat}>Chat</button>
      <button type="button" onClick={()=>onNavigate('newAgent')}>New agent</button>
      <button type="button" onClick={()=>onNavigate('settings')}>Settings</button>
      <button type="button" aria-pressed={sound} disabled={soundPending} onClick={onSound}>Sound</button>
    </nav>
    <div className="aylul-panel-body">
      <section className="aylul-bound-card" aria-label="Bound agent">
        {bound?<>
          <AgentPreview color={bound.color} outfit={bound.outfit} size={112}/>
          <h2 dir="auto">{bound.name}</h2>
          {bound.description&&<p dir="auto">{bound.description}</p>}
          <span className="aylul-panel-status">{status}</span>
          <div className="aylul-bound-actions">
            <button type="button" className="aylul-panel-primary" onClick={onChat}>Talk</button>
            <button type="button" aria-label="Agent settings" onClick={()=>onNavigate('agentSettings')}>Settings</button>
            <button type="button" onClick={()=>onNavigate('bound')}>Open</button>
          </div>
        </>:<>
          <h2>Choose an agent</h2>
          <p>Pick who AYLUL talks as from the agents beside this card.</p>
        </>}
      </section>
      <div className="aylul-panel-agents" role="group" aria-label="Agents">
        {panel.agents.map(agent=><AgentTile key={agent.slug} agent={agent} disabled={selecting} onSelect={()=>onSelect(agent.slug)}/>)}
        <button type="button" className="aylul-agent-tile aylul-agent-new" aria-label="Create a new agent" onClick={()=>onNavigate('newAgent')}><span aria-hidden="true" className="aylul-agent-plus">+</span><span>New agent</span></button>
      </div>
    </div>
    {error&&<p role="alert" className="aylul-panel-error">{error}</p>}
  </section>;
}
