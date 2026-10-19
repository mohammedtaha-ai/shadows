import React from 'react';
import {localInstallation,type AssistantSelection} from '../platform/assistant';

export function useAssistantLocation(){
  const [source,setSource]=React.useState<'local'|'remote'|null>(null);
  const [installation,setInstallation]=React.useState<string|null>(null);
  const [error,setError]=React.useState('');
  const generation=React.useRef(0);
  const refresh=React.useCallback(async()=>{
    const current=++generation.current;
    try{
      const id=(await localInstallation())??null;if(generation.current!==current)return;
      setInstallation(id);setError('');
      setSource(old=>old??(localStorage.getItem('aylul-assistant-location')==='remote'?'remote':localStorage.getItem('aylul-assistant-location')==='local'?'local':id?'local':'remote'));
    }catch{
      if(generation.current===current){setInstallation(null);setError('Local assistant connection is unavailable. Check the daemon, then refresh.');setSource(old=>old??'local');}
    }
  },[]);
  React.useEffect(()=>{void refresh();return()=>{generation.current++;};},[refresh]);
  const select=(value:string)=>{if(value!=='local'&&value!=='remote')return;localStorage.setItem('aylul-assistant-location',value);setSource(value);};
  const selection:AssistantSelection|null=source==='local'?(installation?{source:'local',installationId:installation}:null):source==='remote'?{source:'remote'}:null;
  return {source,selection,installation,error,refresh,select};
}

export function AssistantLocation({location}:{location:ReturnType<typeof useAssistantLocation>}){
  return <div className="assistant-location">
    <label>Assistant location <select aria-label="Assistant location" value={location.source??''} onChange={event=>location.select(event.target.value)}>
      {location.source===null&&<option value="">Checking connections…</option>}
      <option value="local" disabled={!location.installation}>Local{location.installation?'':' — unavailable'}</option>
      <option value="remote">Server — account backend</option>
    </select></label>
    <button type="button" onClick={()=>void location.refresh()}>Refresh connections</button>
    {location.error&&<p role="status">{location.error}</p>}
  </div>;
}
