import { Tabs } from '@base-ui/react/tabs'
import { useState } from 'react'
import { designKeys, getOutcomes, getParts, type Outcome, type Part, useDesignPages } from '@/api/design'
import type { ThreadPlanningAnchor } from '@/api/planning-context'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { ErrorLine } from '../error-line'

interface PickerProps {
  projectId: string
  pending: boolean
  error: Error | null
  onChoose: (anchor: ThreadPlanningAnchor) => void
}

export function AnchorPicker({ error, onClose, ...props }: PickerProps & { onClose: () => void }) {
  return <Dialog open onOpenChange={open => { if (!open && !props.pending) onClose() }}>
    <DialogContent showCloseButton={!props.pending}>
      <DialogTitle>Choose a planning anchor</DialogTitle>
      <Tabs.Root defaultValue="part">
        <Tabs.List className="mb-3 flex gap-2 border-b border-border" aria-label="Anchor type">
          <Tabs.Tab value="part" className="px-3 py-2 text-muted-foreground data-active:font-medium data-active:text-foreground">Parts</Tabs.Tab>
          <Tabs.Tab value="outcome" className="px-3 py-2 text-muted-foreground data-active:font-medium data-active:text-foreground">Outcomes</Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel value="part"><Hierarchy {...props} kind="part" /></Tabs.Panel>
        <Tabs.Panel value="outcome"><Hierarchy {...props} kind="outcome" /></Tabs.Panel>
      </Tabs.Root>
      {error && <ErrorLine error={error} />}
    </DialogContent>
  </Dialog>
}

type LevelProps = Omit<PickerProps, 'error'> & { kind: ThreadPlanningAnchor['kind'] }
type Parent = Pick<Part, 'id'> & { title: string }

function Hierarchy(props: LevelProps) {
  const [path, setPath] = useState<Parent[]>([])
  const parent = path.at(-1)
  return <>
    {parent && <div className="mb-2 flex items-center gap-2">
      <Button size="sm" variant="ghost" onClick={() => setPath(path.slice(0, -1))}>Back</Button>
      <bdi className="text-muted-foreground">{parent.title}</bdi>
    </div>}
    <Level key={parent?.id ?? 'root'} {...props} parent={parent?.id} onBrowse={item => setPath([...path, item])} />
  </>
}

function Level({ kind, projectId, pending, onChoose, parent, onBrowse }: LevelProps & {
  parent?: string
  onBrowse: (parent: Parent) => void
}) {
  const collection = kind === 'part' ? 'parts' : 'outcomes'
  const pages = useDesignPages<Part | Outcome>(
    designKeys.page(projectId, collection, parent).slice(0, -1),
    after => kind === 'part' ? getParts(projectId, parent, after) : getOutcomes(projectId, parent, after),
  )
  return <div className="max-h-[55vh] space-y-2 overflow-y-auto">
    {pages.pending && <p>Loading {collection}…</p>}
    {pages.error && <ErrorLine error={pages.error} />}
    {!pages.pending && !pages.error && pages.items.length === 0 && <p className="text-muted-foreground">No {collection} at this level.</p>}
    <ul className="space-y-1">
      {pages.items.map(({ id, content: { title } }) => <li key={id} className="flex items-center gap-2">
        <Button className="min-w-0 flex-1 justify-start" variant="secondary" size="sm"
          aria-label={`Browse ${title}`} onClick={() => onBrowse({ id, title })}><bdi className="truncate">{title}</bdi></Button>
        <Button variant="outline" size="sm" disabled={pending}
          aria-label={`Select ${title}`} onClick={() => onChoose({ kind, id })}>Select</Button>
      </li>)}
    </ul>
    {pages.next && <Button size="sm" variant="secondary" disabled={pages.loading} onClick={() => void pages.more()}>Load more</Button>}
  </div>
}
