// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { ApiError } from '@/api/error'
import { type Attempt, type TaskRun, taskRunQuery } from '@/api/runs'
import { agentEntry, runningOperation, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'

const PAGE = '/projects/p1/workflows/w1/versions/1/tasks/1'

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

function attempt(extra: Partial<Attempt> = {}): Attempt {
  return {
    id: 'a1', project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, number: 1, thread_id: 't1',
    operation_id: null, model: 'fake-large', state: 'running', started_at: '2026-11-08T10:00:00Z',
    ended_at: null, commit: null, reason: null, report: null, ...extra,
  }
}

function taskRun(extra: Partial<TaskRun> = {}): TaskRun {
  return {
    task: 1, done: false, done_at: null, latest: null, needs: [],
    start: { allowed: true, code: null, reason: null }, suggested_model: 'fake-large', ...extra,
  }
}

function click(element: HTMLElement | undefined): Promise<void> {
  return act(async () => { element?.click() })
}

function modelInput(): HTMLInputElement {
  return document.querySelector<HTMLInputElement>('input[aria-label="Executor model"]')!
}

describe('the task page', () => {
  it('start_sends_the_chosen_model_and_shows_running', async () => {
    let current = taskRun()
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => Response.json(current),
      startAttempt: () => {
        current = taskRun({ latest: attempt({ model: 'fake-small' }), start: { allowed: false, code: 'task_busy', reason: 'running' } })
        return Response.json(current.latest, { status: 201 })
      },
    })))
    await until(() => modelInput() !== null)
    expect(modelInput().value).toBe('fake-large')
    typeInto(modelInput(), 'fake-small')
    await click(a.button('Start'))
    await until(() => a.button('Stop') !== undefined)
    expect(a.bodies).toContainEqual(expect.objectContaining({ model: 'fake-small' }))
    expect(a.text()).toContain('Running')
  })

  it('start_is_disabled_with_the_waiting_tasks', async () => {
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({
        needs: [{ task: 2, title: 'Login screen', done: false }, { task: 3, title: 'Mail', done: true }],
        start: { allowed: false, code: 'waiting', reason: 'T2 is not done yet.' },
      }),
    })))
    await until(() => a.button('Start') !== undefined)
    expect(a.button('Start')?.disabled).toBe(true)
    expect(a.text()).toContain('T2 is not done yet.')
    expect(a.text()).toContain('T2 Login screen')
    expect(document.querySelectorAll('[aria-label="done"]')).toHaveLength(1)
  })

  it('stop_shows_stopped', async () => {
    let current = taskRun({ latest: attempt() })
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => Response.json(current),
      stopAttempt: () => {
        current = taskRun({ latest: attempt({ state: 'stopped', ended_at: '2026-11-08T10:05:00Z' }) })
        return Response.json(current.latest)
      },
    })))
    await until(() => a.button('Stop') !== undefined)
    await click(a.button('Stop'))
    await until(() => a.button('Start') !== undefined)
    expect(a.text()).toContain('Stopped')
  })

  it('a_refused_start_shows_cores_reason_and_keeps_the_model', async () => {
    let reads = 0
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => { reads += 1; return Response.json(taskRun()) },
      startAttempt: () => Response.json({ code: 'dirty_scope', reason: 'backend/ has uncommitted changes.' }, { status: 409 }),
    })))
    await until(() => modelInput() !== null)
    typeInto(modelInput(), 'fake-small')
    await click(a.button('Start'))
    await until(() => a.text().includes('backend/ has uncommitted changes.'))
    expect(modelInput().value).toBe('fake-small')
    await until(() => reads > 1)
  })

  it('diff_opens_the_first_file_and_switches_on_click', async () => {
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({ latest: attempt({ state: 'committed', commit: 'abc123' }) }),
      attemptDiff: [
        { path: 'backend/a.php', added: 1, removed: 1, patch: '@@ -4,2 +4,2 @@\n context\n-old line\n+new line' },
        { path: 'web/b.ts', added: 1, removed: 0, patch: '@@ -1 +1,2 @@\n keep\n+second file line' },
      ],
    })))
    await until(() => a.button('Diff') !== undefined)
    await click(a.button('Diff'))
    await until(() => a.text().includes('new line'))
    expect(a.text()).toContain('old line')
    expect(a.text()).not.toContain('second file line')
    await click(a.button('web/b.ts+1-0'))
    await until(() => a.text().includes('second file line'))
    expect(a.text()).not.toContain('new line')
  })

  it('diff_shows_a_cut_marker_as_plain_text', async () => {
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({ latest: attempt({ state: 'committed', commit: 'abc123' }) }),
      attemptDiff: [{ path: 'big.txt', added: 1, removed: 0, patch: '@@ -1 +1,2 @@\n keep\n+<b>x</b>\n… patch cut at 1000 lines' }],
    })))
    await until(() => a.button('Diff') !== undefined)
    await click(a.button('Diff'))
    await until(() => a.text().includes('… patch cut at 1000 lines'))
    expect(a.text()).toContain('<b>x</b>')
    expect(document.querySelector('b')).toBeNull()
  })

  it('a_committed_task_waits_for_the_planners_judgment', async () => {
    let current = taskRun({
      latest: attempt({ state: 'committed', commit: 'abc123' }),
      start: { allowed: false, code: 'task_busy', reason: 'the Planner is judging' },
    })
    const a = (app = await startApp(PAGE, answers({ taskRun: () => Response.json(current) })))
    await until(() => a.text().includes("Waiting for the Planner's judgment"))
    current = { ...current, done: true, done_at: '2026-11-11T10:00:00Z' }
    await until(() => !a.text().includes("Waiting for the Planner's judgment"))
    expect(a.text()).toContain('Done')
  })

  it('a_task_stopped_on_the_cap_does_not_wait', async () => {
    let reads = 0
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => {
        reads += 1
        return Response.json(taskRun({ latest: attempt({ state: 'committed', commit: 'abc123' }) }))
      },
    })))
    await until(() => a.text().includes('Committed'))
    const before = reads
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 2500)) })
    expect(a.text()).not.toContain("Waiting for the Planner's judgment")
    expect(reads).toBe(before)
  })

  it('a_task_marked_done_does_not_wait', async () => {
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({ latest: attempt({ state: 'committed', commit: 'abc123' }), done: true }),
    })))
    await until(() => a.text().includes('Committed'))
    expect(a.text()).not.toContain("Waiting for the Planner's judgment")
  })

  it('mark_done_updates_the_header', async () => {
    const a = (app = await startApp(PAGE, answers({
      markDone: (request: Request) => {
        void request
        return Response.json(taskRun({ done: true, done_at: '2026-11-08T11:00:00Z' }))
      },
    })))
    await until(() => a.button('Mark done') !== undefined)
    await click(a.button('Mark done'))
    await until(() => a.button('Mark not done') !== undefined)
    expect(a.bodies).toContainEqual(expect.objectContaining({ done: true }))
  })

  it('a_retry_after_a_network_error_reuses_the_command_id', async () => {
    let tries = 0
    const a = (app = await startApp(PAGE, answers({
      startAttempt: () => {
        tries += 1
        if (tries === 1) throw new TypeError('network down')
        return Response.json(attempt(), { status: 201 })
      },
    })))
    await until(() => a.button('Start') !== undefined)
    await click(a.button('Start'))
    await until(() => tries === 1 && a.text().includes('not reachable'))
    await click(a.button('Start'))
    await until(() => tries === 2)
    const sent = a.bodies.filter((b) => typeof b === 'object' && b !== null && 'model' in b) as unknown as { command_id: string }[]
    expect(sent).toHaveLength(2)
    expect(sent[1]!.command_id).toBe(sent[0]!.command_id)
  })

  it('the_report_reads_the_attempts_again_until_the_newest_ends', async () => {
    let current = [attempt()]
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => Response.json(taskRun({ latest: current[0]! })),
      attempts: () => Response.json(current),
    })))
    await until(() => a.button('Report') !== undefined)
    await click(a.button('Report'))
    await until(() => a.text().includes('Attempt 1'))
    current = [attempt({ state: 'committed', commit: 'abc123', ended_at: '2026-11-08T10:05:00Z' })]
    await until(() => a.text().includes('abc123'))
  })

  it('the_report_shows_the_executors_report_as_plain_text', async () => {
    const report = 'Added the login form.\n  The T2 test waits for T2.\n<b>not markup</b>'
    const committed = attempt({ state: 'committed', commit: 'abc123', ended_at: '2026-11-08T10:05:00Z', report })
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({ latest: committed }),
      attempts: [committed],
    })))
    await until(() => a.button('Report') !== undefined)
    await click(a.button('Report'))
    await until(() => a.text().includes('abc123'))
    const shown = document.querySelector<HTMLElement>('[aria-label="Executor report"]')
    expect(shown?.textContent).toBe(report)
    expect(shown?.querySelector('b')).toBeNull()
  })

  it('the_report_shows_nothing_extra_without_a_report', async () => {
    const committed = attempt({ state: 'committed', commit: 'abc123', ended_at: '2026-11-08T10:05:00Z' })
    const a = (app = await startApp(PAGE, answers({
      taskRun: taskRun({ latest: committed }),
      attempts: [committed],
    })))
    await until(() => a.button('Report') !== undefined)
    await click(a.button('Report'))
    await until(() => a.text().includes('abc123'))
    expect(document.querySelector('[aria-label="Executor report"]')).toBeNull()
  })

  it('a_task_core_cannot_find_shows_its_message_and_reads_once', async () => {
    let reads = 0
    const a = (app = await startApp(PAGE, answers({
      taskRun: () => {
        reads += 1
        return Response.json({ code: 'INVALID_COMMAND', message: 'not found: task or attempt' }, { status: 404 })
      },
    })))
    await until(() => a.text().includes('not found: task or attempt'))
    expect(a.text()).toContain('INVALID_COMMAND')
    expect(a.text()).not.toContain('Reading the task')
    expect(reads).toBe(1)
    // The app's client retries by default; the read itself must not retry Core's answer.
    const retry = taskRunQuery('w1', 1, 1).retry
    expect(typeof retry).toBe('function')
    const shouldRetry = retry as (failures: number, error: Error) => boolean
    expect(shouldRetry(0, new ApiError({ kind: 'daemon', status: 404, code: 'INVALID_COMMAND' }, 'not found'))).toBe(false)
    expect(shouldRetry(0, new ApiError({ kind: 'unreachable' }, 'The daemon is not reachable'))).toBe(true)
  })

  it('the_run_keeps_reading_after_the_commit_until_the_turn_ends', async () => {
    let ended = false
    const routes = answers({
      taskRun: taskRun({
        latest: attempt({ state: 'committed', commit: 'abc123', operation_id: 'op1', ended_at: '2026-11-08T10:05:00Z' }),
      }),
    })
    routes['GET /api/threads/t1/operations'] = () =>
      Response.json([{ ...runningOperation(), finished_at: ended ? '2026-11-08T10:06:00Z' : null }])
    routes['GET /api/threads/t1/entries'] = () => Response.json(ended
      ? [userEntry('u1', 'Do task T1.'), agentEntry('a1', 'Committed; the T2 test waits for T2.')]
      : [userEntry('u1', 'Do task T1.')])
    const a = (app = await startApp(PAGE, routes))
    await until(() => a.button('Run') !== undefined)
    await click(a.button('Run'))
    await until(() => a.text().includes('Do task T1.'))
    ended = true
    await until(() => a.text().includes('the T2 test waits for T2.'))
  })
})
