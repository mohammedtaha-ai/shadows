// Read-only layer places, served items and contract inspection.
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { PartContent } from '@/api/design'
import { visionQuery } from '@/api/design'
import { Button } from '@/components/ui/button'

/** A child the part groups: a saved part opens on the map, a proposed one is an anchor on its card. */
type GroupedChild = {title:string} & ({part:string} | {anchor:string})

export function MapPartDetails({content,projectId,groups=[]}: {
  content:PartContent;projectId:string;groups?:GroupedChild[]
}) {
  const vision = useQuery(visionQuery(projectId))
  const items = Object.values(vision.data?.content ?? {}).flat()
  const [panel,setPanel] = useState<{title:string;text:string} | null>(null)
  return <div className="space-y-2 text-sm">
    {content.kind && <p>Kind: {content.kind}</p>}
    <p>Serves: {(content.serves ?? []).length ? (content.serves ?? []).map(id =>
      <span key={id} title={items.find(i => i.id === id)?.text ?? 'Removed vision item'} className="mr-2">{id}</span>) : 'No linked items'}</p>
    <p>Folder: {content.folder || 'None'}</p>
    {groups.length > 0 ? <p>Groups: {groups.map(group => 'part' in group
      ? <Link key={group.part} to="/projects/$projectId/workspace" params={{projectId}}
        search={{view:'map',part:group.part}} className="mr-2 text-accent-line hover:underline"
        dir="auto">{group.title}</Link>
      : <a key={group.anchor} href={`#${group.anchor}`} className="mr-2 text-accent-line hover:underline"
        dir="auto">{group.title}</a>)}</p>
      : (content.layers ?? []).length > 0 && <ul aria-label={`${content.title} layers`} className="space-y-1">
        {content.layers?.map(layer => <li key={layer.layer} className="break-words">
          <span className="font-medium">{layer.layer}</span> · {layer.not_used != null
            ? `Not used: ${layer.not_used}` : layer.place || 'Place not recorded'}
        </li>)}
      </ul>}
    {content.contract && <Button size="sm" variant="secondary" onClick={() => setPanel({
      title:`${content.title} contract`,text:`${content.contract?.header ?? ''}\n${content.contract?.rules.join('\n') ?? ''}`,
    })}>View contract</Button>}
    {panel && <aside role="dialog" aria-label={panel.title}
      className="fixed right-0 top-0 z-50 h-screen w-[min(40rem,90vw)] overflow-auto border-l bg-card p-5 shadow-xl">
      <Button size="sm" onClick={() => setPanel(null)}>Close inspection</Button>
      <h3 className="my-3 font-medium" dir="auto">{panel.title}</h3>
      <pre dir="auto" className="whitespace-pre-wrap break-words">{panel.text}</pre>
    </aside>}
  </div>
}
