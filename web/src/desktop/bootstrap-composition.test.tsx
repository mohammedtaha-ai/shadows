// @vitest-environment happy-dom
import {expect,it,vi} from 'vitest'
import {bootApplication,type ConnectionView} from './runtime'
import {prepareDesktop} from './bootstrap'

const {render,unmount,listen,snapshot,setOrigin}=vi.hoisted(()=>({
  render:vi.fn(),unmount:vi.fn(),listen:vi.fn(),snapshot:vi.fn(),setOrigin:vi.fn(),
}))
vi.mock('react-dom/client',()=>({createRoot:()=>({render,unmount})}))
vi.mock('./native',()=>({connectionBridge:{listen,snapshot},openExternal:vi.fn()}))
vi.mock('../api/daemon-origin',()=>({setDaemonOrigin:setOrigin}))

it('renders injected connection UI while waiting, then fixes origin before mounting the application',async()=>{
  document.body.innerHTML='<div id="root"></div>'
  const starting:ConnectionView={revision:1,state:'starting',configurationId:'config',origin:null,error:null}
  let receive!:(view:ConnectionView)=>void
  const order:string[]=[]
  listen.mockImplementation(async(callback)=>{receive=callback;return()=>{}})
  snapshot.mockResolvedValue(starting)
  setOrigin.mockImplementation(()=>{order.push('origin')})
  unmount.mockImplementation(()=>{order.push('unmount')})
  const controls=<div>Injected controls</div>
  const boot=bootApplication(true,()=>prepareDesktop(controls),async()=>{order.push('mount')})
  await vi.waitFor(()=>expect(render).toHaveBeenCalled())
  expect(render.mock.calls[0]![0].props.children).toContain(controls)
  expect(order).toEqual([])
  const ready:ConnectionView={...starting,revision:2,state:'ready',origin:'http://127.0.0.1:43381'}
  snapshot.mockResolvedValue(ready)
  receive(ready)
  await boot
  expect(setOrigin).toHaveBeenCalledWith(ready.origin)
  expect(order).toEqual(['origin','unmount','mount'])
})
