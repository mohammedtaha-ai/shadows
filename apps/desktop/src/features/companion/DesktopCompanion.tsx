import React from 'react';
import {CompanionCanvas} from './CompanionCanvas';
import {SpeechBubble} from './SpeechBubble';
import {DEFAULT_SETTINGS,outfitOf} from './catalog.ts';
import type {MotionInput} from './types.ts';
import {connectCompanion,dragCompanion,openMainFromCompanion,reportCompanionFailure,setCompanionInput,setCompanionPanel,selectCompanionAgent,type MainTarget,type CompanionState} from '../../platform/companion';
import {createDesktopGesture} from './desktop-gesture.ts';
import {createNativePresentation} from './native-presentation.ts';
import {CompanionComposer} from './CompanionComposer';
import {CompanionPanel} from './panel/CompanionPanel';
import {createTones} from './audio.ts';
import './desktop.css';
export function DesktopCompanion():React.JSX.Element {
  const [state,setState]=React.useState<CompanionState|null>(null),[ready,setReady]=React.useState(false),[error,setError]=React.useState(''),[canvasError,setCanvasError]=React.useState('');
  const owner=React.useRef(createNativePresentation());
  const tones=React.useRef<ReturnType<typeof createTones>|null>(null);
  const [sound,setSound]=React.useState(false),[soundPending,setSoundPending]=React.useState(false),[selecting,setSelecting]=React.useState(false);
  React.useEffect(()=>{tones.current=createTones();return()=>{tones.current?.dispose();tones.current=null;};},[]);
  const [presentation,setPresentation]=React.useState(()=>owner.current.read(Date.now()));
  React.useEffect(()=>{tones.current?.play(presentation.expression==='success'?'success':'state');},[presentation.expression]);
  const gestures=React.useMemo(()=>createDesktopGesture({drag:dragCompanion,open:()=>setCompanionPanel(true).then(()=>{}),fail:setError}),[]);
  React.useEffect(()=>()=>gestures.dispose(),[gestures]);
  React.useEffect(()=>{if(ready)return connectCompanion(setState,setError,true);},[ready]);
  React.useEffect(()=>{if(canvasError)void reportCompanionFailure().catch(()=>setError('Unable to report AYLUL failure.'));},[canvasError]);
  React.useEffect(()=>{
    owner.current.receive(state?.presence,Date.now());
    let timer:ReturnType<typeof setTimeout>|undefined;
    const refresh=()=>{
      clearTimeout(timer);
      const now=Date.now(),value=owner.current.read(now);
      setPresentation(value);
      if(value.message)timer=setTimeout(refresh,Math.max(1,owner.current.deadline()-now));
    };
    refresh();document.addEventListener('visibilitychange',refresh);
    return ()=>{clearTimeout(timer);document.removeEventListener('visibilitychange',refresh);};
  },[state?.presence]);
  const input:MotionInput={color:state?.color,viewport:{width:0,height:0,dpr:1},presentation,settings:DEFAULT_SETTINGS,outfit:outfitOf(state?.outfit),scene:'studio',pointer:null,direction:null,perch:null};
  const point=(event:React.PointerEvent)=>({pointerId:event.pointerId,button:event.button,x:event.clientX,y:event.clientY});
  const setInput=(open:boolean)=>{void setCompanionInput(open).catch(()=>setError('Unable to change AYLUL input.'));};
  const composer=state?.composer;
  const panel=state?.panel;
  React.useEffect(()=>{if(!panel?.open)return;const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();void setCompanionPanel(false).catch(()=>setError('Unable to close the panel.'));}};document.addEventListener('keydown',escape);return()=>document.removeEventListener('keydown',escape);},[panel?.open]);
  const navigate=(target:MainTarget)=>{void openMainFromCompanion(target).catch(()=>setError('Unable to open AYLUL.'));};
  const select=(agent:string)=>{setSelecting(true);setError('');void selectCompanionAgent(agent).catch(()=>setError('Unable to select this agent.')).finally(()=>setSelecting(false));};
  const toggleSound=()=>{if(sound){tones.current?.disable();setSound(false);return;}setSoundPending(true);void tones.current?.enable().then(()=>setSound(true)).catch(()=>setError('Sound is unavailable.')).finally(()=>setSoundPending(false));};
  return <div className={`aylul-desktop${composer?.open?' aylul-input-open':''}${panel?.open?' aylul-panel-open':''}`}>
   <div className="aylul-artwork" onPointerDown={event=>{if(event.button===0){event.currentTarget.setPointerCapture(event.pointerId);gestures.down(point(event));}}} onPointerMove={event=>gestures.move(point(event))} onPointerUp={event=>gestures.up(point(event))} onPointerCancel={()=>gestures.cancel()} onLostPointerCapture={()=>gestures.cancel()} onClickCapture={event=>{event.preventDefault();event.stopPropagation();}} onKeyDown={event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();void setCompanionPanel(true).catch(()=>setError('Unable to open the panel.'));}}}>
    {canvasError?<p role="alert">{canvasError}</p>:<CompanionCanvas input={input} paused={state?.paused??false} renderAllowed={state?.renderAllowed??false} onReady={()=>setReady(true)} onFailure={setCanvasError} actionLabel="Open AYLUL panel or drag AYLUL"/>}
   </div>
    {!canvasError&&<><SpeechBubble message={presentation.message} compact/>
     <button type="button" className="aylul-live-status" aria-label="Talk to AYLUL" aria-expanded={composer?.open??false} onClick={()=>setInput(!composer?.open)}><span role="status" aria-label="AYLUL status">{presentation.statusText}</span></button>
     {composer&&<CompanionComposer key={composer.scope??'unbound'} view={composer} busy={['thinking','working','speaking'].includes(presentation.expression)} onClose={()=>setInput(false)}/>}
    </>}
    {!canvasError&&panel?.open&&<CompanionPanel panel={panel} status={presentation.statusText} onClose={()=>{void setCompanionPanel(false).catch(()=>setError('Unable to close the panel.'));}} onNavigate={navigate} onChat={()=>setInput(true)} onSelect={select} selecting={selecting} sound={sound} soundPending={soundPending} onSound={toggleSound} error={error}/>}
    {!canvasError&&!panel?.open&&error&&<p className="aylul-operation-error" role="alert">{error}</p>}
  </div>;
}
