// One job: telling a person whether the daemon answers, and what to do when
// it does not. Reachability is a real call through the typed client — the
// project list — never a guess.

import { useQuery } from '@tanstack/react-query'
import { CircleAlert } from 'lucide-react'
import type { ReactNode } from 'react'
import { DAEMON_URL } from '@/api/client'
import { ApiError } from '@/api/error'
import { projectsQuery } from '@/api/queries'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'

const DAEMON_HOST = new URL(DAEMON_URL).host

/** A dot and a line: connecting, connected, or why not. */
export function ConnectionIndicator() {
  const { status, error } = useQuery(projectsQuery)

  const [dot, label] =
    status === 'success'
      ? ['bg-success', 'Connected']
      : status === 'pending'
        ? ['animate-pulse bg-faint-foreground', 'Connecting']
        : isUnreachable(error)
          ? ['bg-destructive', 'Not reachable']
          : ['bg-destructive', 'Daemon error']

  return (
    <p className="flex items-center gap-2 text-xs text-muted-foreground" aria-live="polite">
      <span aria-hidden className={`size-2 shrink-0 rounded-full ${dot}`} />
      <span className="truncate">
        {label} · {DAEMON_HOST}
      </span>
    </p>
  )
}

/** What went wrong reaching the daemon, and the command that fixes the usual case. */
export function DaemonProblem({
  error,
  retry,
  retrying,
}: {
  error: Error
  retry: () => void
  retrying: boolean
}) {
  return (
    <Alert className="max-w-lg border-destructive-border p-4">
      <CircleAlert className="text-destructive" />
      <AlertTitle className="text-destructive-foreground">
        {isUnreachable(error)
          ? `Shadows can't reach its daemon at ${DAEMON_URL}`
          : `The daemon at ${DAEMON_URL} answered with an error`}
      </AlertTitle>
      <AlertDescription className="mt-2 space-y-3 text-muted-foreground">
        {isUnreachable(error) ? (
          <>
            <p>Start it in a terminal:</p>
            <Command>shadows serve --harness &lt;path to claude.exe&gt;</Command>
            <p>
              Already running? It must allow this page's origin, which it does by default for
              Vite's dev server; otherwise start it with:
            </p>
            <Command>--allow-origin {window.location.origin}</Command>
          </>
        ) : (
          <p>
            {error instanceof ApiError && error.problem.kind === 'daemon' && (
              <code className="mr-2 font-mono text-destructive-foreground">
                {error.problem.code}
              </code>
            )}
            {error.message}
          </p>
        )}
        <Button variant="outline" size="sm" onClick={retry} disabled={retrying}>
          {retrying ? 'Trying…' : 'Try again'}
        </Button>
      </AlertDescription>
    </Alert>
  )
}

function Command({ children }: { children: ReactNode }) {
  return (
    <pre className="overflow-x-auto rounded-md border border-border bg-input-background px-3 py-2 font-mono text-xs text-foreground">
      {children}
    </pre>
  )
}

function isUnreachable(error: Error | null): boolean {
  return error instanceof ApiError && error.problem.kind === 'unreachable'
}
