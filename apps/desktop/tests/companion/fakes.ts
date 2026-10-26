export function installComposerNative() {
 const win=window as any;const callbacks=new Map<number,Function>();let next=0,listener=0;
 let state:any={revision:1,visible:true,renderAllowed:true,paused:false,error:null,presence:{expression:'idle',status:'Ready',preview:null},composer:{open:false,revision:1,scope:'one',context:'ctx',agent:'agent-a'}};
 const fx:any={calls:[],errors:[],delay:false,publish:(change:any)=>{state={...state,...change,revision:state.revision+1};callbacks.get(listener)?.({event:'companion-state',payload:state});}};
 win.__composerFx=fx;win.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
 win.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'companion'},currentWebview:{label:'companion'}},transformCallback(fn:Function){callbacks.set(++next,fn);return next;},unregisterCallback(id:number){callbacks.delete(id);},async invoke(command:string,args:any){
  fx.calls.push({command,args});
  if(command==='plugin:event|listen'){listener=args.handler;return listener;}if(command==='plugin:event|unlisten')return;
  if(command==='companion_ready'||command==='companion_state')return state;
  if(command==='companion_input'){fx.publish({composer:{...state.composer,open:args.open,revision:state.composer.revision+1}});return state;}
  if(command==='companion_send'){if(fx.delay)return new Promise((_resolve,reject)=>{fx.reject=reject;});if(fx.errors.length)throw fx.errors.shift();return {threadId:'thread',operationId:'operation'};}
  if(command==='open_main_from_companion'||command==='drag_companion')return;
  if(command==='companion_failed')return state;
  throw Error('Unexpected fixture command');
 }};
}
