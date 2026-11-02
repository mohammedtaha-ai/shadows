import { useEffect, useState } from 'react'
import { stopTask } from '@/api/sessions'
import { Panel } from '@/components/ui/panel'
import type { BackgroundTask } from '@/stream'
import { ErrorLine } from '../error-line'

export function BackgroundTasks({ threadId, tasks, onClose }: { threadId: string; tasks: readonly BackgroundTask[]; onClose: () => void }) {
  const [now, setNow] = useState(0)
  const [pending, setPending] = useState<string[]>([])
  const [error, setError] = useState<Error | null>(null)
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [])
  const stop = async (id: string) => {
    setPending(previous => [...previous, id]); setError(null)
    try { await stopTask(threadId, id) } catch (e) { setError(e instanceof Error ? e : new Error(String(e))) }
    finally { setPending(previous => previous.filter(value => value !== id)) }
  }
  const running = tasks.filter(task => task.state === 'running')
  const finished = tasks.filter(task => task.state !== 'running')
  return <aside aria-label="Background tasks" className="w-80 shrink-0 overflow-auto border-l border-border p-4">
    <div className="flex items-center justify-between"><h2 className="font-medium">Background tasks</h2><button type="button" onClick={onClose} aria-label="Close background tasks">Close</button></div>
    {error && <ErrorLine error={error} />}
    {[...running, ...finished].map(task => {
      const title = task.name || task.description
      const seconds = Math.max(0, Math.floor(((task.ended_at ?? (now || task.started_at)) - task.started_at) / 1000))
      return <Panel key={task.id} size="sm" className="mt-3">
        <p dir="auto">{title}</p><p className="text-xs text-muted-foreground">{task.state} · {Math.floor(seconds / 60)}m {seconds % 60}s</p>
        {task.state === 'running' && task.can_stop && <button type="button" disabled={pending.includes(task.id)} onClick={() => void stop(task.id)} aria-label={`Stop ${title}`} className="text-sm text-muted-foreground hover:text-foreground">Stop {title}</button>}
        {task.summary && <p className="text-xs text-muted-foreground">{task.summary}</p>}
      </Panel>
    })}
  </aside>
}
