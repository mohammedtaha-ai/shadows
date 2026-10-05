// The save/reload lifecycle shared by part and outcome editors.
import { type UseQueryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { editDesign, type DesignEdit } from '@/api/design'
import { ApiError } from '@/api/error'

type ViewBasis = { revision: number; plans: string[] }
type DesignOp = DesignEdit['ops'][number]
type EditorQuery<Data, Key extends readonly unknown[]> =
  Pick<UseQueryOptions<Data, Error, Data, Key>, 'queryKey' | 'queryFn'>

export interface DesignEditorConfig<View extends ViewBasis, Content> {
  entityKind: 'Part' | 'Outcome'
  empty: Content
  idOf: (view: View) => string
  contentOf: (view: View) => Content
  detailQuery: (projectId: string, id: string) => EditorQuery<View, string[]>
  listQuery: (projectId: string) => EditorQuery<{ revision: number }, (string | null)[]>
  contentOp: (
    id: string,
    content: Content,
    position: { parent: string | null; before: string | null } | null,
  ) => DesignOp
  relations?: {
    read: (view: View) => string[]
    ops: (id: string, selected: string[], previous: string[]) => DesignOp[]
  }
}

export function useDesignEditor<View extends ViewBasis, Content>(
  config: DesignEditorConfig<View, Content>,
  { projectId, saved, destination, onCreated }: {
    projectId: string
    saved?: View
    destination?: string
    onCreated?: () => void
  },
) {
  const client = useQueryClient()
  const root = useQuery({ ...config.listQuery(projectId), enabled: saved === undefined })
  const [base, setBase] = useState(saved)
  const [draft, setDraft] = useState(saved ? config.contentOf(saved) : config.empty)
  const [linked, setLinked] = useState(saved?.plans ?? [])
  const [related, setRelated] = useState(saved ? config.relations?.read(saved) ?? [] : [])
  const [moving, setMoving] = useState(false)
  const [order, setOrder] = useState({ destination, before: '' })
  const before = order.destination === destination ? order.before : ''
  const setBefore = (value: string) => setOrder({ destination, before: value })
  const [conflict, setConflict] = useState(false)
  const [reloading, setReloading] = useState(false)
  const [reloadError, setReloadError] = useState<Error | null>(null)
  const [createRevision, setCreateRevision] = useState<number | undefined>(root.data?.revision)
  if (createRevision === undefined && root.data) setCreateRevision(root.data.revision)
  const [id] = useState(() => saved ? config.idOf(saved) : crypto.randomUUID())
  const attempt = useRef<Attempt | null>(null)
  const dirty = JSON.stringify(draft) !== JSON.stringify(base ? config.contentOf(base) : config.empty)
    || JSON.stringify(linked) !== JSON.stringify(base?.plans ?? [])
    || JSON.stringify(related) !== JSON.stringify(base ? config.relations?.read(base) ?? [] : [])
    || moving
  if (saved && base && saved.revision > base.revision && !dirty) {
    setBase(saved)
    setDraft(config.contentOf(saved))
    setLinked(saved.plans)
    setRelated(config.relations?.read(saved) ?? [])
  }
  const newer = conflict || (saved !== undefined && base !== undefined && saved.revision > base.revision)
    || (base === undefined && createRevision !== undefined && root.data !== undefined && root.data.revision > createRevision)

  const adopt = (view: View) => {
    setBase(view)
    setDraft(config.contentOf(view))
    setLinked(view.plans)
    setRelated(config.relations?.read(view) ?? [])
    setMoving(false)
    setBefore('')
  }
  const save = useMutation({
    mutationFn: (body: DesignEdit) => editDesign(projectId, body),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
      if (!base) {
        attempt.current = null
        setConflict(false)
        onCreated?.()
        return
      }
      const view = await client.fetchQuery({ ...config.detailQuery(projectId, id), staleTime: 0 })
      // A failed authoritative read must leave the committed command replayable.
      attempt.current = null
      setConflict(false)
      adopt(view)
    },
    onError: error => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'REVISION_CONFLICT') {
        setConflict(true)
        void client.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
      }
    },
  })

  const submit = () => {
    const position = { parent: destination ?? null, before: before || null }
    const ops: DesignOp[] = [config.contentOp(id, draft, base ? null : position)]
    if (base && moving) {
      ops.push({ kind: config.entityKind === 'Part' ? 'PartMove' : 'OutcomeMove', id, ...position })
    }
    const anchor = { kind: config.entityKind, id }
    for (const plan of linked.filter(p => !base?.plans.includes(p))) {
      ops.push({ kind: 'PlanLinkPut', anchor, plan })
    }
    for (const plan of (base?.plans ?? []).filter(p => !linked.includes(p))) {
      ops.push({ kind: 'PlanLinkRemove', anchor, plan })
    }
    if (config.relations) {
      ops.push(...config.relations.ops(id, related, base ? config.relations.read(base) : []))
    }
    const expected_revision = base?.revision ?? createRevision
    if (expected_revision === undefined) return
    const request = { expected_revision, ops }
    attempt.current = attemptFor(attempt.current, request)
    save.mutate({ ...request, command_id: attempt.current.commandId })
  }

  const reload = async () => {
    setReloading(true)
    setReloadError(null)
    try {
      if (!base) {
        const page = await client.fetchQuery({ ...config.listQuery(projectId), staleTime: 0 })
        setCreateRevision(page.revision)
        setDraft(config.empty)
        setLinked([])
        setRelated([])
        setBefore('')
      } else {
        adopt(await client.fetchQuery({ ...config.detailQuery(projectId, id), staleTime: 0 }))
      }
      setConflict(false)
      attempt.current = null
      save.reset()
    } catch (error) {
      setReloadError(error instanceof Error ? error : new Error(String(error)))
    } finally {
      setReloading(false)
    }
  }

  return {
    base, draft, setDraft, linked, setLinked, related, setRelated,
    moving, setMoving, before, setBefore, id, dirty, newer,
    createRevision, reloading, reloadError, save, submit, reload,
  }
}
