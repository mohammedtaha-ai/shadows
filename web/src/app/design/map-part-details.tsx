// Read-only folders, served items, starter files and contract inspection.
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { PartContent } from '@/api/design'
import { visionQuery } from '@/api/design'
import { Button } from '@/components/ui/button'

export function MapPartDetails({content,projectId}: {content:PartContent;projectId:string}) {
  const vision = useQuery(visionQuery(projectId))
  const items = Object.values(vision.data?.content ?? {}).flat()
  const [panel,setPanel] = useState<{title:string;text:string} | null>(null)
  return <div className="space-y-2 text-sm">
    {content.kind && <p>Kind: {content.kind}</p>}
    <p>Serves: {(content.serves ?? []).length ? (content.serves ?? []).map(id =>
      <span key={id} title={items.find(i => i.id === id)?.text ?? 'Removed vision item'} className="mr-2">{id}</span>) : 'No linked items'}</p>
    <p>Folders: {(content.folders ?? []).join(', ') || 'None'}</p>
    {(content.files ?? []).map(file => <Button key={file.path} size="sm" variant="secondary"
      onClick={() => setPanel({title:file.path,text:file.content})}>{file.path}</Button>)}
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
