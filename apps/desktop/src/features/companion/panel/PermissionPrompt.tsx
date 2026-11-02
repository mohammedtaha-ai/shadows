import React from 'react';
import {answerCompanionPermission} from '../../../platform/companion';
import type {WaitingRequest} from '../../../platform/companion-state';

const REFUSALS:Record<string,string>={
  permission_settled:'Already answered',
  permission_not_found:'No longer waiting',
  permission_option_not_offered:'That option is no longer offered',
  companion_core_unavailable:'AYLUL could not be reached. Press the option again to retry.',
};
function codeOf(reason:unknown):string{
  if(typeof reason==='string')return reason;
  if(reason&&typeof reason==='object'&&typeof (reason as {message?:unknown}).message==='string')return (reason as {message:string}).message;
  return '';
}

export function PermissionPrompt({request}:{request:WaitingRequest}){
  // A retry after an unreachable Core repeats the same command; any other outcome ends it.
  const attempt=React.useRef<{requestId:string;optionId:string;commandId:string}|null>(null);
  const [pending,setPending]=React.useState(false);
  const [line,setLine]=React.useState('');
  React.useEffect(()=>{attempt.current=null;setLine('');},[request.requestId]);
  const answer=async(optionId:string)=>{
    if(pending)return;
    const current=attempt.current;
    const commandId=current&&current.requestId===request.requestId&&current.optionId===optionId?current.commandId:crypto.randomUUID();
    attempt.current={requestId:request.requestId,optionId,commandId};
    setPending(true);setLine('');
    try{
      await answerCompanionPermission(request.threadId,request.requestId,optionId,commandId);
      attempt.current=null;
    }catch(reason){
      const code=codeOf(reason);
      if(code!=='companion_core_unavailable')attempt.current=null;
      setLine(REFUSALS[code]??'The answer was not accepted.');
    }finally{setPending(false);}
  };
  return <section className="aylul-permission" aria-label="Permission request">
    <p className="aylul-permission-title" dir="auto">{request.title}</p>
    <div className="aylul-permission-options">
      {request.options.map(option=><button key={option.optionId} type="button" disabled={pending} onClick={()=>{void answer(option.optionId);}}>{option.name}</button>)}
    </div>
    {line&&<p role="status" className="aylul-permission-line">{line}</p>}
  </section>;
}
