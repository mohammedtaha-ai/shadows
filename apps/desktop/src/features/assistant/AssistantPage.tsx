import React from 'react';
import {assistant,type AssistantSnapshot,type AssistantConversation} from '../../platform/assistant';
import type {AuthView} from '../../platform/auth';
import './assistant.css';
const waiting=(state:string)=>['submitting','running','stopping','unknown'].includes(state);
function message(code:unknown){const value=String(code);return value.includes('verification_refused')?'Please sign in to continue.':value.includes('request_conflict')?'This conversation already has a pending request. Refresh before sending again.':value.includes('request_refused')?'The message could not be sent. Keep it under 4 KB.':'Cannot reach the assistant. Your conversation is saved; you can retry.';}
export function AssistantPage({auth}:{auth:AuthView|null}){
 const principal=auth?.state==='signed_in'?auth.principal:null;
 const account=principal?.accountId??'',session=principal?.sessionId??'';
 const [conversations,setConversations]=React.useState<AssistantConversation[]>([]);
 const [selected,setSelected]=React.useState('');
 const [snapshot,setSnapshot]=React.useState<AssistantSnapshot|null>(null);
 const [draft,setDraft]=React.useState(''),[error,setError]=React.useState(''),[busy,setBusy]=React.useState(false);
 const [ready,setReady]=React.useState(false);
 const scope=account+':'+session+':'+selected,live=React.useRef(scope);live.current=scope;
 const pending=React.useRef<{id:string;text:string;conversation:string}|null>(null);
 const sending=React.useRef(false),pollFailed=React.useRef(false),initial=React.useRef(crypto.randomUUID()),tail=React.useRef<HTMLDivElement>(null);
 React.useEffect(()=>{
  setSnapshot(null);setConversations([]);setSelected('');setDraft('');setError('');setReady(false);pending.current=null;
  if(!account)return;
  let active=true;
  void assistant.list().then(async list=>{
   if(!active)return;setConversations(list.conversations);
   const saved=localStorage.getItem('aylul-conversation:'+account);
   if(list.conversations.length){setSelected(list.conversations.find(c=>c.id===saved)?.id??list.conversations[0]!.id);}
   else{const first=await assistant.create(initial.current);if(active){setSelected(first.conversation.id);setSnapshot(first);setConversations([first.conversation]);}}
  }).catch(e=>{if(active)setError(message(e));});
  return()=>{active=false;};
 },[account,session]);
 React.useEffect(()=>{
  if(!account||!selected)return;localStorage.setItem('aylul-conversation:'+account,selected);
  const current=scope;let active=true,refreshing=false;
  const refresh=async()=>{if(refreshing)return;refreshing=true;
   try{const [status,data]=await Promise.all([assistant.status(),assistant.snapshot(selected)]);
    if(!active||live.current!==current)return;setReady(status.state==='ready');setSnapshot(data);
    if(pollFailed.current){pollFailed.current=false;setError('');}
    if(pending.current&&data.turns.some(turn=>turn.commandId===pending.current?.id)){pending.current=null;setDraft('');}
   }catch(e){if(active&&live.current===current){setReady(false);pollFailed.current=true;setError(message(e));}}finally{refreshing=false;}
  };
  void refresh();const timer=setInterval(()=>{if(!document.hidden)void refresh();},1500);
  return()=>{active=false;clearInterval(timer);};
 },[account,session,selected]);
 React.useEffect(()=>{tail.current?.scrollIntoView({block:'nearest'});},[snapshot?.turns.map(t=>t.id+t.state+t.output.length).join('|')]);
 async function createNew(){if(busy)return;const current=scope;setBusy(true);pollFailed.current=false;setError('');
  try{const data=await assistant.create(crypto.randomUUID());if(live.current!==current)return;pending.current=null;setDraft('');setConversations(list=>[data.conversation,...list]);setSelected(data.conversation.id);setSnapshot(data);}
  catch(e){if(live.current===current)setError(message(e));}finally{setBusy(false);}
 }
 async function send(){if(sending.current||!selected||!draft.trim()||!ready)return;
  if(new TextEncoder().encode(draft).length>4096){setError('Keep your message under 4 KB.');return;}
  const current=scope;const intent=pending.current?.conversation===selected&&pending.current.text===draft?pending.current:{id:crypto.randomUUID(),text:draft,conversation:selected};
  pending.current=intent;sending.current=true;setBusy(true);pollFailed.current=false;setError('');
  try{const data=await assistant.send(selected,intent.id,intent.text);if(live.current!==current)return;setSnapshot(data);setDraft('');pending.current=null;}
  catch(e){if(live.current===current)setError(message(e));}finally{sending.current=false;setBusy(false);}
 }
 async function control(turnId:string,recover=false){const current=scope;pollFailed.current=false;setError('');setBusy(true);
  try{const data=await (recover?assistant.recover(selected,turnId):assistant.stop(selected,turnId));if(live.current===current)setSnapshot(data);}
  catch(e){if(live.current===current)setError(message(e));}finally{setBusy(false);}
 }
 const active=snapshot?.turns.find(t=>waiting(t.state));
 return <section className="assistant" aria-label="Assistant conversation">
  <div className="assistant-heading"><h2>Chat with AYLUL</h2><button disabled={!account||busy} onClick={()=>void createNew()}>New conversation</button></div>
  {!account?<p>Sign in below to start a conversation.</p>:<>
   <label>Conversation <select aria-label="Conversation" value={selected} disabled={busy} onChange={e=>{pending.current=null;setDraft('');setSelected(e.target.value);setSnapshot(null);setError('');}}>{conversations.map(c=><option key={c.id} value={c.id}>{c.title} · {new Date(c.createdAt).toLocaleString()}</option>)}</select></label>
   <p className="assistant-connection" role="status">{ready?'Connected': 'The assistant is currently unavailable. Your conversation is saved.'}</p>
   <div className="assistant-messages" role="log" aria-label="Messages" aria-live="polite">
    {!snapshot?.turns.length&&<p className="assistant-empty">What would you like to talk about?</p>}
    {snapshot?.turns.map(turn=><React.Fragment key={turn.id}>
     <article className="assistant-message user"><strong>You</strong><p dir="auto">{turn.input}</p></article>
     <article className="assistant-message reply"><strong>AYLUL</strong><p dir="auto">{turn.output||({submitting:'Sending…',running:'Thinking…',unknown:'The result is uncertain. Check the original request before sending again.',stopping:'Stopping…',cancelled:'Response stopped.',error:'The response could not be completed.',done:'No text was returned.'}[turn.state])}</p>
      {turn.state==='unknown'&&<button disabled={busy} onClick={()=>void control(turn.id,true)}>Recover original request</button>}
      {['running','submitting','stopping'].includes(turn.state)&&<button disabled={busy||turn.state==='stopping'} onClick={()=>void control(turn.id)}>Stop response</button>}
     </article>
    </React.Fragment>)}<div ref={tail}/>
   </div>
   {snapshot?.truncated&&<p>Older messages or long responses were shortened in this view.</p>}
   <form onSubmit={e=>{e.preventDefault();void send();}}><label htmlFor="assistant-message">Message</label>
    <textarea id="assistant-message" value={draft} disabled={busy||!!active} placeholder="Message AYLUL…" rows={3} onChange={e=>setDraft(e.target.value)} onKeyDown={e=>{if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();void send();}}}/>
    <div className="assistant-compose-footer"><span>Enter to send · Shift+Enter for a new line</span><button type="submit" disabled={!ready||busy||!!active||!draft.trim()}>Send message</button></div>
   </form>
  </>}
  {error&&<p role="alert">{error}</p>}
 </section>;
}
