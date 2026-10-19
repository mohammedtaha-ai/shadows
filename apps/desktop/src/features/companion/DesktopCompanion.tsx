import React from 'react';
import {CompanionCanvas} from './CompanionCanvas';
import {DEFAULT_SETTINGS} from './catalog.ts';
import type {MotionInput} from './types.ts';
import {connectCompanion,dragCompanion,openMainFromCompanion,reportCompanionFailure,type CompanionState} from '../../platform/companion';
import {createDesktopGesture} from './desktop-gesture.ts';
import './desktop.css';
export function DesktopCompanion():React.JSX.Element {
  const [state,setState]=React.useState<CompanionState|null>(null),[ready,setReady]=React.useState(false),[error,setError]=React.useState(''),[canvasError,setCanvasError]=React.useState('');
  const gestures=React.useMemo(()=>createDesktopGesture({drag:dragCompanion,open:openMainFromCompanion,fail:setError}),[]);
  React.useEffect(()=>()=>gestures.dispose(),[gestures]);
  React.useEffect(()=>{if(ready)return connectCompanion(setState,setError,true);},[ready]);
  React.useEffect(()=>{if(canvasError)void reportCompanionFailure().catch(()=>setError('Unable to report AYLUL failure.'));},[canvasError]);
  const input:MotionInput={viewport:{width:0,height:0,dpr:1},presentation:{expression:'idle',statusText:'Idle',message:null},settings:DEFAULT_SETTINGS,outfit:'default',scene:'studio',pointer:null,direction:null,perch:null};
  const point=(event:React.PointerEvent)=>({pointerId:event.pointerId,button:event.button,x:event.clientX,y:event.clientY});
  return <div className="aylul-desktop" onPointerDown={event=>{if(event.button===0){event.currentTarget.setPointerCapture(event.pointerId);gestures.down(point(event));}}} onPointerMove={event=>gestures.move(point(event))} onPointerUp={event=>gestures.up(point(event))} onPointerCancel={()=>gestures.cancel()} onLostPointerCapture={()=>gestures.cancel()} onClickCapture={event=>{event.preventDefault();event.stopPropagation();}} onKeyDown={event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();void openMainFromCompanion().catch(()=>setError('Unable to open NYLRA.'));}}}>
    {canvasError?<p role="alert">{canvasError}</p>:<CompanionCanvas input={input} paused={state?.paused??false} renderAllowed={state?.renderAllowed??false} onReady={()=>setReady(true)} onFailure={setCanvasError} actionLabel="Open NYLRA or drag AYLUL"/>}
    {!canvasError&&error&&<p className="aylul-operation-error" role="alert">{error}</p>}
  </div>;
}
