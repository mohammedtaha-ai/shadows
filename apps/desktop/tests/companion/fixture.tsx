// Test-only entry: never imported by the application.
import React from "react";
import '../../src/features/companion/companion.css';
const source = "/src/features/companion/";
if(new URLSearchParams(location.search).has('nativeVisibility')) {
  Promise.all([import('react-dom/client'),import('../../src/features/companion/CompanionCanvas'),import('../../src/features/companion/catalog')]).then(([{createRoot},{CompanionCanvas},{DEFAULT_SETTINGS}])=>{
    function Fixture(){
      const [allowed,setAllowed]=React.useState(!new URLSearchParams(location.search).has('denied'));
      const [paused,setPaused]=React.useState(false),[ready,setReady]=React.useState(false),[mounted,setMounted]=React.useState(true),[expression,setExpression]=React.useState('idle');
      const input:any={viewport:{width:0,height:0,dpr:1},presentation:{expression,statusText:expression,message:null},settings:DEFAULT_SETTINGS,outfit:'default',scene:'studio',pointer:null,direction:null,perch:null};
      return <><button onClick={()=>setAllowed(false)}>Deny drawing</button><button onClick={()=>setAllowed(true)}>Allow drawing</button><button onClick={()=>setPaused(true)}>Pause fixture</button><button onClick={()=>setExpression('blocked')}>Change input</button><button onClick={()=>setMounted(false)}>Unmount canvas</button><p>{ready?'Canvas ready':'Initializing'}</p>{mounted&&<div style={{width:240,height:240}}><CompanionCanvas input={input} paused={paused} renderAllowed={allowed} onReady={()=>setReady(true)} onFailure={()=>setReady(false)}/></div>}</>;
    }
    createRoot(document.getElementById('root')!).render(<Fixture/>);
  });
}
async function render(expression = "idle", outfit = "default") {
  try {
    const { renderCharacter } = await import(
      /* @vite-ignore */ source + "renderer/character.ts"
    );
    const { DEFAULT_SETTINGS } = await import(
      /* @vite-ignore */ source + "catalog.ts"
    );
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 256;
    const ctx = canvas.getContext("2d")!;
    const frame = {
      cx: 128,
      cy: 145,
      scale: 0.7,
      lookX: 0,
      lookY: 0,
      tilt: 0,
      time: 0.25,
      earringAngle: 0,
      blinkProgress: 0,
      expression,
      outfit,
      mini: false,
      perchingHands: false,
      sleepingMarks: [
        { offset: 0, size: 12 },
        { offset: 0.33, size: 16 },
        { offset: 0.66, size: 20 },
      ],
      notificationPulse: 0,
    };
    renderCharacter(ctx, frame, DEFAULT_SETTINGS);
    const first = canvas.toDataURL();
    const pixels = ctx.getImageData(0, 0, 256, 256).data;
    const visible = pixels.filter((v, i) => i % 4 === 3 && v > 0).length;
    const corner = pixels[3];
    ctx.clearRect(0, 0, 256, 256);
    renderCharacter(ctx, frame, DEFAULT_SETTINGS);
    return {
      visible,
      corner,
      deterministic: first === canvas.toDataURL(),
      image: first,
      defaults: DEFAULT_SETTINGS,
    };
  } catch (error) {
    return { error: String(error) };
  }
}
Object.assign(window, { fixture: { render } });
if (new URLSearchParams(location.search).has("shell")) {
  const callLog = document.createElement("pre");
  callLog.dataset.testid = "fixture-ipc-calls";
  document.body.append(callLog);
  import("@tauri-apps/api/mocks").then(async ({ mockIPC,mockWindows }) => {
    mockWindows('main');
    const failed=new URLSearchParams(location.search).has('companionFailed');
    const hookSession=new URLSearchParams(location.search).has('hookSession');
    let pending=hookSession?[{id:'request-one',kind:'request',sessionId:'session-one',project:'C:/synthetic/project',tool:'Read',target:'C:/synthetic/project/a.ts',reason:null,proposedRule:null,at:'2026-10-17T10:00:00Z',held:true}]:[];
    const signedIn=new URLSearchParams(location.search).has('signedIn');
    const stamp='2026-10-19T10:00:00.000Z';
    const mk=(id:string,title:string,at=stamp)=>({id,title,createdAt:at,updatedAt:at});
    const turnOf=(n:number,input:string,output:string)=>({id:`00000000-0000-4000-8000-00000000000${n}`,commandId:`10000000-0000-4000-8000-00000000000${n}`,input,output,state:'done',createdAt:stamp,updatedAt:stamp});
    const fx:any={
      principal:{accountId:'acct-a',sessionId:'sess-a',deviceId:'dev-1'},
      data:{
        'acct-a':{conversations:[mk('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1','Alpha plan','2026-10-19T09:00:00.000Z'),mk('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2','Beta notes','2026-10-19T09:30:00.000Z')],turns:{'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1':[turnOf(1,'Alpha question','Alpha answer')],'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2':[turnOf(2,'Beta question','Beta answer')]}},
        'acct-b':{conversations:[mk('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1','Other account chat')],turns:{'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1':[turnOf(3,'Other question','Other answer')]}},
      },
      calls:[] as any[],offline:false,dropSendResponse:false,gate:null as null|Promise<void>,
    };
    fx.data.local=structuredClone(fx.data['acct-a']);
    fx.data.local.turns['aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1']=[turnOf(1,'Local question','Local answer')];
    (window as any).__fx=fx;
    async function assistantCall(r:Record<string,string>,owner=fx.principal.accountId){
      const mine=fx.data[owner];
      fx.calls.push({...r,account:owner});
      if(fx.offline)throw Error('backend_unavailable');
      const snap=(id:string)=>({conversation:mine.conversations.find((c:any)=>c.id===id),turns:[...(mine.turns[id]??[])],truncated:false});
      switch(r.action){
        case 'status':return {state:'ready'};
        case 'list':return {conversations:[...mine.conversations]};
        case 'create':{const id=`cccccccc-cccc-4ccc-8ccc-${String(mine.conversations.length+1).padStart(12,'0')}`;mine.conversations.unshift(mk(id,'New conversation'));mine.turns[id]=[];return snap(id);}
        case 'snapshot':{const out=snap(r.conversationId!);if(fx.gate)await fx.gate;return out;}
        case 'send':{if(!mine.turns[r.conversationId!].some((t:any)=>t.commandId===r.commandId))mine.turns[r.conversationId!].push({...turnOf(9,r.text!,'Reply to '+r.text),commandId:r.commandId,id:crypto.randomUUID()});if(fx.sendGate)await fx.sendGate;if(fx.dropSendResponse){fx.dropSendResponse=false;throw Error('backend_unavailable');}return snap(r.conversationId!);}
      }
      throw Error('request_refused');
    }
    let companion={revision:1,visible:!failed,renderAllowed:!failed,paused:false,error:failed?'companion_unavailable':null as string|null};
    mockIPC((command, args) => {
      callLog.textContent += `${command} ${JSON.stringify(args)}\n`;
      if (command === "auth_action" && args?.action === "status" && signedIn)
        return {state:"signed_in",hasEnrollment:true,principal:fx.principal,providerLogout:null};
      if (command === "assistant_sources") return new URLSearchParams(location.search).has('localAssistant')?'00000000-0000-4000-8000-000000000099':null;
      if (command === "assistant_local_call") return assistantCall(args?.request as Record<string,string>,'local');
      if (command === "assistant_call") return assistantCall(args?.request as Record<string,string>);
      if (command === "auth_action" && args?.action === "status")
        return {
          state: "signed_out",
          hasEnrollment: false,
          principal: null,
          providerLogout: null,
        };
      if (command === "hide_main") return;
      if(command==='aylul_call'){
        if(new URLSearchParams(location.search).has('sidecarUnavailable'))throw Error('sidecar_unavailable');
        if(args?.method==='state')return {sessions:hookSession?[{id:'session-one',agent:'claude_code',project:'C:/synthetic/project',state:'waiting',startedAt:'2026-10-17T10:00:00Z',updatedAt:'2026-10-17T10:00:00Z',lastStep:null,lastMessage:null}]:[],pending,hooksInstalled:true};
        if(args?.method==='request.answer'){pending=[];return null;}
      }
      if (command==='companion_state')return companion;
      if(command==='set_companion_visible'){
        if(new URLSearchParams(location.search).has('hideRefused'))throw Error('refused');
        companion={...companion,revision:companion.revision+1,visible:args?.visible as boolean,renderAllowed:args?.visible as boolean,error:null};return companion;
      }
      if(command==='set_companion_paused'){companion={...companion,revision:companion.revision+1,paused:args?.paused as boolean};return companion;}
      throw new Error("Unexpected fixture command");
    },{shouldMockEvents:true});
    await import("../../src/app/main");
  });
}
if (new URLSearchParams(location.search).has("presence")) {
  Promise.all([
    import("react"),
    import("react-dom/client"),
    import(/* @vite-ignore */ source + "Companion.tsx"),
  ])
    .then(([React, { createRoot }, { Companion }]) => {
      function Fixture() {
        const [mounted, setMounted] = React.useState(true);
        const [expression, setExpression] = React.useState("idle");
        const [message, setMessage] = React.useState<any>(null);
        const [width, setWidth] = React.useState(400);
        return (
          <>
            <button onClick={() => setMounted((v) => !v)}>Mount toggle</button>
            <button
              onClick={() =>
                setExpression((v) => (v === "idle" ? "blocked" : "idle"))
              }
            >
              Change expression
            </button>
            <button
              onClick={() =>
                setMessage({
                  id: String(performance.now()),
                  text: "Owner message",
                })
              }
            >
              New message
            </button>
            <button
              onClick={() =>
                setMessage({
                  id: "unsafe",
                  text:
                    "<img onerror=alert(1)> Hey! جاهز لمساعدتك " +
                    "A".repeat(150),
                })
              }
            >
              Mixed text
            </button>
            <button onClick={() => setWidth((v) => (v === 0 ? 300 : 0))}>
              Zero size
            </button>
            <div style={{ width, maxWidth: "100%" }}>
              {mounted && (
                <Companion
                  presentation={{ expression, statusText: expression, message }}
                />
              )}
            </div>
          </>
        );
      }
      createRoot(document.getElementById("root")!).render(
        <React.StrictMode>
          <Fixture />
        </React.StrictMode>,
      );
    })
    .catch((error) => {
      document.getElementById("root")!.textContent = String(error);
    });
}
if (new URLSearchParams(location.search).has("studio")) {
  Promise.all([
    import("react-dom/client"),
    import(/* @vite-ignore */ source + "studio/CompanionStudio.tsx"),
    import(/* @vite-ignore */ source + "Companion.tsx"),
  ])
    .then(([{ createRoot }, { CompanionStudio }, { Companion }]) => {
      function Fixture() {
        const [open, setOpen] = React.useState(true);
        return (
          <>
            <Companion />
            <button onClick={() => setOpen((v) => !v)}>Studio toggle</button>
            {open && <CompanionStudio />}
          </>
        );
      }
      createRoot(document.getElementById("root")!).render(
        <React.StrictMode>
          <Fixture />
        </React.StrictMode>,
      );
    })
    .catch((error) => {
      document.getElementById("root")!.textContent = String(error);
    });
}
