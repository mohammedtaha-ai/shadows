import React from 'react';
import {CompanionCanvas} from './CompanionCanvas';
import {SpeechBubble} from './SpeechBubble';
import {DEFAULT_SETTINGS,outfitOf} from './catalog.ts';
import type {MotionInput} from './types.ts';
import {connectCompanion,dragCompanion,openMainFromCompanion,reportCompanionFailure,setCompanionInput,type CompanionState} from '../../platform/companion';
import {createDesktopGesture} from './desktop-gesture.ts';
import {createNativePresentation} from './native-presentation.ts';
import {CompanionComposer} from './CompanionComposer';
import './desktop.css';
export function DesktopCompanion():React.JSX.Element {
  const [state,setState]=React.useState<CompanionState|null>(null),[ready,setReady]=React.useState(false),[error,setError]=React.useState(''),[canvasError,setCanvasError]=React.useState('');
  const owner=React.useRef(createNativePresentation());
  const [presentation,setPresentation]=React.useState(()=>owner.current.read(Date.now()));
  const gestures=React.useMemo(()=>createDesktopGesture({drag:dragCompanion,open:openMainFromCompanion,fail:setError}),[]);
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
  return <div className={`aylul-desktop${composer?.open?' aylul-input-open':''}`}>
   <div className="aylul-artwork" onPointerDown={event=>{if(event.button===0){event.currentTarget.setPointerCapture(event.pointerId);gestures.down(point(event));}}} onPointerMove={event=>gestures.move(point(event))} onPointerUp={event=>gestures.up(point(event))} onPointerCancel={()=>gestures.cancel()} onLostPointerCapture={()=>gestures.cancel()} onClickCapture={event=>{event.preventDefault();event.stopPropagation();}} onKeyDown={event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();void openMainFromCompanion().catch(()=>setError('Unable to open AYLUL.'));}}}>
    {canvasError?<p role="alert">{canvasError}</p>:<CompanionCanvas input={input} paused={state?.paused??false} renderAllowed={state?.renderAllowed??false} onReady={()=>setReady(true)} onFailure={setCanvasError} actionLabel="Open NYLRA or drag AYLUL"/>}
   </div>
    {!canvasError&&<><SpeechBubble message={presentation.message} compact/>
     <button type="button" className="aylul-live-status" aria-label="Talk to AYLUL" aria-expanded={composer?.open??false} onClick={()=>setInput(!composer?.open)}><span role="status" aria-label="AYLUL status">{presentation.statusText}</span></button>
     {composer&&<CompanionComposer key={composer.scope??'unbound'} view={composer} busy={['thinking','working','speaking'].includes(presentation.expression)} onClose={()=>setInput(false)}/>}
    </>}
    {!canvasError&&error&&<p className="aylul-operation-error" role="alert">{error}</p>}
  </div>;
}
