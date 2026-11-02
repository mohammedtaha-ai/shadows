// One job: the open session's background tasks beside the conversation, each
// running one with its own stop.

import { X } from 'lucide-react'
import { useEffect, useState } from 'react'
import { stopTask } from '@/api/sessions'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import type { BackgroundTask } from '@/stream'
import { ErrorLine } from '../error-line'

export function BackgroundTasks({ threadId, tasks, onClose }: { threadId: string; tasks: readonly BackgroundTask[]; onClose: () => void }) {
  const [now, setNow] = useState(() => Date.now())
  const [pending, setPending] = useState<readonly string[]>([])
  const [error, setError] = useState<Error | null>(null)
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [])
  // The stream's next `tasks` frame is what marks the task stopped.
  const stop = async (id: string) => {
    setPending(previous => [...previous, id])
    setError(null)
    try {
      await stopTask(threadId, id)
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)))
    } finally {
      setPending(previous => previous.filter(value => value !== id))
    }
  }
  const running = tasks.filter(task => task.state === 'running')
  const finished = tasks.filter(task => task.state !== 'running')
  const item = (task: BackgroundTask) => {
    const title = task.name || task.description
    const seconds = Math.max(0, Math.floor(((task.ended_at ?? now) - task.started_at) / 1000))
    const stopping = pending.includes(task.id)
    return <li key={task.id}>
      <Panel size="sm">
        <p dir="auto" className="text-sm text-foreground">{title}</p>
        <p className="text-xs text-muted-foreground">{task.state} · {Math.floor(seconds / 60)}m {seconds % 60}s</p>
        {task.summary && <p dir="auto" className="text-xs text-muted-foreground">{task.summary}</p>}
        {task.state === 'running' && task.can_stop && (
          <Button type="button" variant="outline" size="xs" className="mt-2" disabled={stopping}
            aria-label={`Stop ${title}`} onClick={() => void stop(task.id)}>
            {stopping ? 'Stopping…' : 'Stop'}
          </Button>
        )}
      </Panel>
    </li>
  }
  return <aside aria-label="Background tasks" className="flex w-80 shrink-0 flex-col border-l border-border bg-card">
    <header className="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
      <h2 className="text-sm font-semibold text-foreground">Background tasks</h2>
      <Button type="button" variant="ghost" size="icon-xs" aria-label="Close background tasks" onClick={onClose}>
        <X aria-hidden />
      </Button>
    </header>
    <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
      {error && <ErrorLine error={error} />}
      {running.length > 0 && <section aria-label="Running">
        <h3 className="mb-2 text-xs font-semibold text-muted-foreground">Running</h3>
        <ul className="space-y-2">{running.map(item)}</ul>
      </section>}
      {finished.length > 0 && <section aria-label="Finished">
        <h3 className="mb-2 text-xs font-semibold text-muted-foreground">Finished</h3>
        <ul className="space-y-2">{finished.map(item)}</ul>
      </section>}
    </div>
  </aside>
}
