import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { attemptFor, type Attempt } from '@/api/command-id'
import { groupKeys, sendToGroup } from '@/api/groups'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { MentionMenu, type Mentionable } from './mention-menu'

export function GroupComposer({ groupId, members }: { groupId: string; members: readonly Mentionable[] }) {
  const client = useQueryClient()
  const [text, setText] = useState('')
  const [caret, setCaret] = useState(0)
  const [dismissed, setDismissed] = useState(false)
  const input = useRef<HTMLTextAreaElement>(null)
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const send = useMutation({
    mutationFn: () => {
      attempt.current = attemptFor(attempt.current, { groupId, text })
      return sendToGroup(groupId, text, attempt.current.commandId)
    },
    onSuccess: () => {
      attempt.current = null
      setText('')
      setCaret(0)
      void client.invalidateQueries({ queryKey: groupKeys.list })
      void client.invalidateQueries({ queryKey: groupKeys.detail(groupId) })
      void client.invalidateQueries({ queryKey: groupKeys.messages(groupId) })
    },
    onSettled: () => { locked.current = false },
  })
  const mention = /(?:^|[^\p{L}\p{N}_@-])@([a-z0-9_-]*)$/iu.exec(text.slice(0, caret))
  const options = mention && !dismissed && !send.isPending
    ? members.filter(member => member.slug !== 'mohammed' && member.slug.startsWith(mention[1].toLowerCase()))
    : []

  function submit() {
    if (locked.current || text.trim() === '') return
    locked.current = true
    send.mutate()
  }

  function pick(slug: string) {
    if (!mention) return
    const start = caret - mention[1].length - 1
    const end = caret + (/^[a-z0-9_-]*/i.exec(text.slice(caret))?.[0].length ?? 0)
    const completed = `@${slug} `
    setText(text.slice(0, start) + completed + text.slice(end))
    setCaret(start + completed.length)
    requestAnimationFrame(() => {
      input.current?.focus()
      input.current?.setSelectionRange(start + completed.length, start + completed.length)
    })
  }

  return (
    <form className="space-y-2 border-t border-border p-4" onSubmit={event => { event.preventDefault(); submit() }}>
      {options.length > 0 && <MentionMenu members={options} onPick={pick} />}
      <div className="flex items-end gap-2">
        <textarea ref={input} aria-label="Group message" placeholder="Write to the group…" dir="auto"
          className="min-h-12 flex-1 resize-y rounded-lg border border-border bg-input p-2"
          value={text} readOnly={send.isPending}
          onChange={event => { setText(event.target.value); setCaret(event.target.selectionStart); setDismissed(false) }}
          onSelect={event => setCaret(event.currentTarget.selectionStart)}
          onKeyDown={event => {
            if (event.nativeEvent.isComposing) return
            if (event.key === 'Escape' && options.length > 0) { event.stopPropagation(); setDismissed(true) }
            if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); submit() }
          }} />
        <Button type="submit" disabled={send.isPending || text.trim() === ''}>Send message</Button>
      </div>
      {send.error && <ErrorLine error={send.error} />}
    </form>
  )
}
