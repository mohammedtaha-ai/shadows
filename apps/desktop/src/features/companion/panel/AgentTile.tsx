import React from 'react';
import type {PanelAgent} from '../../../platform/companion';
import {DEFAULT_SETTINGS,outfitOf} from '../catalog.ts';
import {createMotion,readFrame} from '../animation/index.ts';
import {renderScene} from '../renderer/index.ts';

// One still frame per agent: a grid of up to 64 animation loops would cost more than the panel is worth.
// The renderer's ground shadow is sized for the 240 px companion, so the frame is drawn there and scaled by CSS.
const FRAME=240;
export function AgentPreview({color,outfit,size}:{color:string;outfit:string;size:number}){
  const canvas=React.useRef<HTMLCanvasElement>(null);
  React.useEffect(()=>{
    const element=canvas.current,context=element?.getContext('2d');
    if(!element||!context)return;
    const viewport={width:FRAME,height:FRAME,dpr:1};
    const settings={...DEFAULT_SETTINGS,showPedestal:false};
    const input={viewport,settings,color,outfit:outfitOf(outfit),scene:'studio' as const,
      presentation:{expression:'idle' as const,statusText:'',message:null},pointer:null,direction:null,perch:null};
    renderScene(context,readFrame(createMotion(viewport,()=>0.5),input),settings);
  },[color,outfit]);
  return <canvas ref={canvas} width={FRAME} height={FRAME} style={{width:size,height:size}} aria-hidden="true"/>;
}

export function AgentTile({agent,onSelect,disabled=false}:{agent:PanelAgent;onSelect:()=>void;disabled?:boolean}){
  return <button type="button" className="aylul-agent-tile" aria-label={agent.name} aria-pressed={agent.bound} disabled={disabled} onClick={onSelect}>
    <AgentPreview color={agent.color} outfit={agent.outfit} size={72}/>
    <span dir="auto">{agent.name}</span><small>{agent.bound?'Bound':'Select'}</small>
  </button>;
}
