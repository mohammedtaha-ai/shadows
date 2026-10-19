// Explicit browser-only native-boundary substitute. Never imported by production.
import React from 'react';
import {createRoot} from 'react-dom/client';
import {AssistantPage} from '../src/features/assistant/AssistantPage';
import '../src/app/shell.css';
const grant=JSON.parse(sessionStorage.getItem('assistant-fixture-grant')??'null');
const root='/__assistant_backend/v1/nylra/assistant';
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={invoke:async(command:string,args:{request:Record<string,string>})=>{
 if(command!=='assistant_call')throw Error('unknown_command');const r=args.request;
 let path=root,method='GET',body:unknown;
 switch(r.action){case 'status':path+='/status';break;case 'list':path+='/conversations';break;
 case 'create':path+='/conversations';method='POST';body={commandId:r.commandId};break;
 case 'snapshot':path+=`/conversations/${r.conversationId}`;break;
 case 'send':path+=`/conversations/${r.conversationId}/turns`;method='POST';body={commandId:r.commandId,text:r.text};break;
 case 'stop':case 'recover':path+=`/conversations/${r.conversationId}/turns/${r.turnId}/${r.action}`;method='POST';body={};break;
 default:throw Error('request_refused');}
 const response=await fetch(path,{method,headers:{authorization:`Bearer ${grant.handle}`,...(body?{'content-type':'application/json'}:{})},...(body?{body:JSON.stringify(body)}:{})});
 if(!response.ok)throw Error(response.status===401?'verification_refused':'backend_unavailable');return response.json();
}};
function Fixture(){
 const [principal,setPrincipal]=React.useState(grant.principal);
 React.useEffect(()=>{const change=(event:Event)=>setPrincipal((event as CustomEvent).detail);window.addEventListener('assistant-fixture-account',change);return()=>window.removeEventListener('assistant-fixture-account',change);},[]);
 return <main><AssistantPage auth={principal?{state:'signed_in',hasEnrollment:true,principal,providerLogout:null}:null}/></main>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
