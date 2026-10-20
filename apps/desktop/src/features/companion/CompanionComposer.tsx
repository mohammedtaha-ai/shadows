import React from 'react';
import type {ComposerView} from '../../platform/companion-state.ts';
import {sendFromCompanion,openMainFromCompanion} from '../../platform/companion';
import {createDrafts,nextAttempt,submitKey,errorMessage,type Attempt} from './composer-model.ts';
type Props={view:ComposerView;busy:boolean;onClose:()=>void};
export function CompanionComposer({view,busy,onClose}:Props):React.JSX.Element|null{
 const drafts=React.useRef(createDrafts());
 const attempt=React.useRef<Attempt|null>(null);
 const active=React.useRef<Attempt|null>(null);
 const latest=React.useRef(view);latest.current=view;
 const input=React.useRef<HTMLTextAreaElement>(null);
 const [text,setText]=React.useState(''),[sending,setSending]=React.useState(false),[error,setError]=React.useState('');
 React.useEffect(()=>{
  drafts.current.drop(view.scope);attempt.current=null;active.current=null;
  setText(view.scope?drafts.current.read(view.scope):'');setSending(false);setError('');
 },[view.scope]);
 React.useEffect(()=>{if(view.open)input.current?.focus();},[view.open,view.scope]);
 const navigate=()=>{void openMainFromCompanion().catch(()=>setError('companion_send_failed'));};
 const retry=error==='companion_send_uncertain'||error==='companion_send_busy';
 const send=async()=>{
  const {scope,context}=latest.current;
  if(!scope||!context||active.current||!text.trim()||busy&&!retry)return;
  const current=nextAttempt(attempt.current,scope,text,()=>crypto.randomUUID());
  attempt.current=current;active.current=current;setSending(true);setError('');
  try{
   await sendFromCompanion(current.text,current.commandId,context);
   if(latest.current.scope===scope&&attempt.current===current){
    drafts.current.write(scope,'');setText('');attempt.current=null;
   }
  }catch(reason){
   if(latest.current.scope===scope&&attempt.current===current)setError(typeof reason==='string'?reason:'companion_send_uncertain');
  }finally{
   if(active.current===current){active.current=null;setSending(false);}
  }
 };
 if(!view.open)return null;
 const unavailable=!view.scope||!view.context||error==='companion_agent_unavailable';
 return <section className="aylul-composer" aria-label="Message AYLUL directly" onKeyDown={event=>{
  const action=submitKey({key:event.key,shiftKey:event.shiftKey,isComposing:event.nativeEvent.isComposing});
  if(action==='close'){event.preventDefault();onClose();}
 }}>
  {unavailable?<button type="button" className="aylul-choose-agent" onClick={navigate}>Choose an agent</button>:<>
   <div className="aylul-compose-row">
    <textarea ref={input} aria-label="Message AYLUL" dir="auto" placeholder="Ask AYLUL…" rows={1} value={text} disabled={sending} readOnly={retry} onChange={event=>{
     setText(event.target.value);drafts.current.write(view.scope!,event.target.value);
    }} onKeyDown={event=>{
     if(submitKey({key:event.key,shiftKey:event.shiftKey,isComposing:event.nativeEvent.isComposing})==='send'){
      event.preventDefault();void send();
     }
    }}/>
    <button type="button" className="aylul-send" aria-label={retry?'Retry message':'Send message'} disabled={sending||!text.trim()||busy&&!retry} onClick={()=>void send()}>
     <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M12 19V5m-5 5 5-5 5 5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/></svg>
    </button>
   </div>
   <div className="aylul-composer-meta"><span className="aylul-agent" title={view.agent??''}>{view.agent}</span><span role="status" aria-label="Message status">{error?errorMessage(error):sending?'Sending…':busy?'AYLUL is answering.':''}</span>
    {['companion_send_setup','companion_thread_invalid','companion_core_unavailable','companion_send_settings'].includes(error)&&<button type="button" onClick={navigate}>Open settings</button>}
   </div>
  </>}
 </section>;
}
