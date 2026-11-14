// Names for the stable ids an agreement stores: a part, and who wrote a version.
import { useQuery } from '@tanstack/react-query'
import type { AgreementVersion } from '@/api/agreements'
import { partQuery } from '@/api/design'
import { threadQuery } from '@/api/threads'
import { SkeletonText } from '@/components/ui/skeleton'

export function PartName({ projectId, id }: { projectId: string; id: string }) {
  const part = useQuery(partQuery(projectId, id))
  return <>{part.data?.part.content.title ?? (part.isError ? 'Part unavailable' : <SkeletonText label="Loading part…" />)}</>
}

export function WriterName({ writer }: { writer: AgreementVersion['writer'] }) {
  if (writer.kind === 'Thread') return <ThreadName id={writer.id} />
  if (writer.kind === 'Grant') return <>External agent</>
  return <>You</>
}

function ThreadName({ id }: { id: string }) {
  const thread = useQuery(threadQuery(id))
  return <>{thread.data?.title ?? (thread.isError ? 'A conversation' : <SkeletonText label="Loading conversation…" />)}</>
}
