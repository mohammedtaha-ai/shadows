import { useLayoutEffect, useRef, useState } from 'react'
import { cn } from 'cn'
import type { Message } from '@/api/groups'
import { Button } from '@/components/ui/button'
import { ReplyText } from '@/app/conversation'

export function GroupLog({ messages, colors }: { messages: readonly Message[]; colors: ReadonlyMap<string, string> }) {
  const ordered = [...messages].sort((a, b) => a.seq - b.seq)
  const latest = ordered.at(-1)?.seq ?? 0
  const log = useRef<HTMLDivElement>(null)
  const [atBottom, setAtBottom] = useState(true)
  const [seen, setSeen] = useState(0)

  useLayoutEffect(() => {
    if (atBottom && log.current) log.current.scrollTop = log.current.scrollHeight
  }, [atBottom, latest])

  const unseen = !atBottom && latest > seen
  return (
    <div className="relative min-h-0 flex-1">
      <div ref={log} aria-label="Group messages" tabIndex={0} className="h-full space-y-3 overflow-y-auto p-4"
        onScroll={event => {
          const element = event.currentTarget
          const nearBottom = element.scrollHeight - element.scrollTop - element.clientHeight <= 24
          setAtBottom(nearBottom)
          if (nearBottom || atBottom) setSeen(latest)
        }}>
        {ordered.map(message => {
          if (message.kind === 'notice') {
            return <p key={message.id} data-message-kind="notice" dir="auto"
              className="text-center text-sm text-muted-foreground">{message.body}</p>
          }
          const speaker = message.speaker
          const mine = speaker.kind === 'person'
          const addressed = !mine && message.to.some(address => address.kind === 'person')
          const name = speaker.kind === 'agent' ? speaker.name : mine ? 'You' : 'Shadows'
          return (
            <article key={message.id} data-message-kind="chat"
              className={cn('max-w-[85%] rounded-xl border border-border bg-card px-4 py-3', mine && 'ms-auto bg-accent/20')}>
              <header className="mb-1 flex flex-wrap gap-2 text-xs">
                <span dir="auto" className="font-semibold"
                  style={{ color: speaker.kind === 'agent' ? colors.get(speaker.slug) : undefined }}>{name}</span>
                {addressed && <span className="text-warning">To you</span>}
              </header>
              {mine ? <p dir="auto" className="whitespace-pre-wrap break-words">{message.body}</p> : <ReplyText text={message.body} />}
            </article>
          )
        })}
      </div>
      {unseen && <Button type="button" className="absolute bottom-3 left-1/2 -translate-x-1/2"
        onClick={() => {
          if (log.current) log.current.scrollTop = log.current.scrollHeight
          setAtBottom(true)
          setSeen(latest)
        }}>New messages</Button>}
    </div>
  )
}
