// One job: the vision as five lists of items (§23.3), edited one item at a time.
import { Link } from '@tanstack/react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { editDesign, type DesignEdit, type VisionContent, type VisionItem, type VisionView } from '@/api/design'
import { ApiError } from '@/api/error'
import { threadsQuery, visionProposalsQuery, visionQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { VISION_FIELDS, fieldLabel } from './vision-fields'
export function ItemOrigin({projectId,item,titles}: {projectId:string;item:VisionItem;titles:Map<string,string>}) {
  const threadId = item.approved_from
  return <small className="text-muted-foreground">{threadId === null || threadId === undefined ? 'added by hand' : <span>from <Link to="/projects/$projectId/threads/$threadId" params={{projectId,threadId}}>{titles.get(threadId) ?? 'a conversation'}</Link></span>}</small>
}
export function VisionEditor({projectId}: {projectId:string}) {
  const vision = useQuery(visionQuery(projectId))
  const proposals = useQuery(visionProposalsQuery(projectId))
  const threads = useQuery(threadsQuery(projectId))
  const pending = proposals.data?.filter(p => p.status === 'Pending' && !p.outdated) ?? []
  const counts = new Map<string,number>()
  for (const p of pending) counts.set(p.thread_id,(counts.get(p.thread_id) ?? 0)+1)
  const titles = new Map(threads.data?.map(t => [t.id,t.title]))
  return <section className="space-y-4">
    {pending.length > 0 && <aside role="status" className="rounded-md border p-3 text-sm">
      {pending.length === 1 ? '1 proposal awaits your approval' : `${pending.length} proposals await your approval`}
      <ul>{[...counts].map(([threadId,count]) => <li key={threadId}><Link to="/projects/$projectId/threads/$threadId" params={{projectId,threadId}}>{titles.get(threadId) ?? 'a conversation'} · {count} pending</Link></li>)}</ul>
    </aside>}
    {proposals.error !== null && <ErrorLine error={proposals.error} />}
    {vision.isPending && <p>Loading vision…</p>}
    {vision.error !== null && <ErrorLine error={vision.error} />}
    {vision.data !== undefined && <Editor key={projectId} projectId={projectId} saved={vision.data} titles={titles} />}
  </section>
}
type ItemOp = Extract<DesignEdit['ops'][number],{kind:'VisionItemAdd'|'VisionItemPut'|'VisionItemRemove'}>
const emptyDrafts = (): Record<keyof VisionContent,string> => ({purpose:'',users:'',goals:'',boundaries:'',technical_direction:''})
function Editor({projectId,saved,titles}: {projectId:string;saved:VisionView;titles:Map<string,string>}) {
  const client = useQueryClient()
  const [base,setBase] = useState(saved)
  const [drafts,setDrafts] = useState(emptyDrafts)
  const [editing,setEditing] = useState<{id:string;text:string}|null>(null)
  const [conflict,setConflict] = useState(false)
  const [refreshError,setRefreshError] = useState<Error|null>(null)
  const [refreshing,setRefreshing] = useState(false)
  const attempt = useRef<Attempt|null>(null)
  const dirty = editing !== null || Object.values(drafts).some(t => t !== '')
  if (saved.revision > base.revision && !dirty && !conflict) setBase(saved)
  const reload = async (discard:boolean) => {
    setRefreshing(true); setRefreshError(null)
    try {
      const v = await client.fetchQuery({...visionQuery(projectId),staleTime:0})
      setBase(v); setConflict(false)
      if (discard) { setEditing(null); setDrafts(emptyDrafts()); attempt.current = null; save.reset() }
    } catch (e) { setRefreshError(e instanceof Error ? e : new Error(String(e))) }
    finally { setRefreshing(false) }
  }
  const save = useMutation({
    mutationFn: (body:DesignEdit) => editDesign(projectId,body),
    onSuccess: async (_result,body) => {
      const op = body.ops[0]
      if (op.kind === 'VisionItemAdd') setDrafts(d => ({...d,[op.field]:''}))
      setEditing(null); attempt.current = null; setConflict(false)
      void client.invalidateQueries({queryKey:['projects',projectId,'design']})
      await reload(false)
    },
    onError: (e) => {
      if (e instanceof ApiError && e.problem.kind === 'daemon' && e.problem.code === 'REVISION_CONFLICT') {
        setConflict(true); void client.invalidateQueries({queryKey:visionQuery(projectId).queryKey})
      }
    },
  })
  const submit = (op:ItemOp) => {
    const request = {expected_revision:base.revision,ops:[op]}
    attempt.current = attemptFor(attempt.current,request)
    save.mutate({...request,command_id:attempt.current.commandId})
  }
  const busy = save.isPending || refreshing
  const needsReload = conflict || saved.revision > base.revision || refreshError !== null
  return <div className="space-y-5">
    {needsReload && <div role="status" className="rounded-md border p-3 text-sm">
      Changed elsewhere or refresh needed. Your unsaved text is kept. Reload replaces it with the saved vision.
      <Button size="sm" variant="secondary" className="ml-3" onClick={() => void reload(true)} disabled={busy}>Reload</Button>
    </div>}
    {VISION_FIELDS.map(field => <section key={field} className="space-y-2">
      <h3 className="text-sm font-medium">{fieldLabel(field)}</h3>
      <ul className="space-y-2">{base.content[field].map(item => <li key={item.id} className="rounded-md border p-3 space-y-2">
        {editing?.id === item.id ? <div className="space-y-2">
          <textarea dir="auto" aria-label={`Edit ${item.id}`} value={editing.text} disabled={busy}
            onChange={e => setEditing({...editing,text:e.target.value})} className="w-full rounded-md border p-2" />
          <Button size="sm" onClick={() => submit({kind:'VisionItemPut',id:item.id,text:editing.text})} disabled={busy || conflict || refreshError !== null || editing.text.trim() === ''}>Save {item.id}</Button>
          <Button size="sm" variant="secondary" onClick={() => {setEditing(null);attempt.current=null;save.reset()}} disabled={busy}>Cancel</Button>
        </div> : <div>
          <p dir="auto" className="whitespace-pre-wrap">{item.id} · {item.text}</p>
          <div className="flex gap-2 mt-2"><Button size="sm" variant="secondary" onClick={() => {setEditing({id:item.id,text:item.text});attempt.current=null;save.reset()}} disabled={busy || editing !== null || conflict || refreshError !== null}>Edit {item.id}</Button>
            <Button size="sm" variant="secondary" onClick={() => submit({kind:'VisionItemRemove',id:item.id})} disabled={busy || editing !== null || conflict || refreshError !== null}>Remove {item.id}</Button></div>
        </div>}
        <ItemOrigin projectId={projectId} item={item} titles={titles} />
      </li>)}</ul>
      <div className="flex gap-2"><input dir="auto" aria-label={`Add ${fieldLabel(field).toLowerCase()}`} value={drafts[field]} onChange={e => setDrafts({...drafts,[field]:e.target.value})} disabled={busy} className="min-w-0 flex-1 rounded-md border p-2 text-sm" />
        <Button size="sm" onClick={() => submit({kind:'VisionItemAdd',field,text:drafts[field]})} disabled={busy || editing !== null || conflict || refreshError !== null || drafts[field].trim() === ''}>Add {fieldLabel(field)}</Button></div>
    </section>)}
    {save.error !== null && <ErrorLine error={save.error} />}
    {refreshError !== null && <ErrorLine error={refreshError} />}
  </div>
}
