type Pointer={pointerId:number;button:number;x:number;y:number};
export function createDesktopGesture(actions:{drag:()=>Promise<void>;open:()=>Promise<void>;fail:(reason:string)=>void}){
  let disposed=false;
  let active:({start:Pointer;dragged:boolean})|null=null;
  const perform=(work:()=>Promise<void>,message:string)=>{void work().catch(()=>{if(!disposed)actions.fail(message)});};
  return {
    down(event:Pointer){if(!disposed&&event.button===0)active={start:event,dragged:false};},
    move(event:Pointer){
      if(disposed||!active||event.pointerId!==active.start.pointerId||active.dragged)return;
      if(Math.hypot(event.x-active.start.x,event.y-active.start.y)<8)return;
      const gesture=active;gesture.dragged=true;
      void actions.drag().catch(()=>{if(!disposed)actions.fail('Unable to move AYLUL.');}).finally(()=>{if(active===gesture)active=null;});
    },
    up(event:Pointer){
      if(disposed||!active||event.pointerId!==active.start.pointerId||event.button!==0)return;
      const gesture=active;active=null;
      if(!gesture.dragged&&Math.hypot(event.x-gesture.start.x,event.y-gesture.start.y)<8)perform(actions.open,'Unable to open the AYLUL panel.');
    },
    cancel(){active=null;},
    dispose(){disposed=true;active=null;},
  };
}
