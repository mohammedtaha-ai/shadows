// One job: the route table of a daemon holding one project (`p1`) with one
// conversation (`t1`) and no plan unless a test gives it one (`w1`), for
// `startApp`. Each answer is the contract fixture
// unless the test overrides it. Test-only.

import type { Operation, PlanListing, Project, ThreadEntry } from '@/api/client'
import type { Answer } from '@/app/test-app'
import {
  agentEntry,
  claudeHarness,
  codexHarness,
  fakeChoices,
  planFixture,
  projectFixture,
  threadFixture,
  userEntry,
} from './contract-fixtures'

export interface Overrides {
  project?: Project
  entries?: ThreadEntry[]
  operations?: Operation[]
  /** `POST /api/threads/t1/session` */
  session?: Answer
  /** `PUT /api/threads/t1/session/model`; by default the session takes the
   * model and answers `fakeChoices` holding it. */
  model?: Answer
  /** `POST /api/threads/t1/turns` */
  start?: Answer
  /** `POST /api/threads/t1/fork` */
  fork?: Answer
  /** `GET /api/threads/t1/context` */
  context?: Answer
  /** `GET /api/projects/p1/workflows`; none by default. */
  plans?: PlanListing[]
  /** `GET /api/workflows/w1`; `planFixture()` by default. */
  plan?: Answer
  /** `POST /api/workflows/w1/approve` */
  approve?: Answer
}

export function answers(o: Overrides = {}): Record<string, Answer> {
  // Kept as the daemon keeps it: a refetch after a PATCH reads what the PATCH
  // saved, as it does against the real daemon.
  let project = o.project ?? projectFixture
  return {
    'GET /api/harnesses': [claudeHarness, codexHarness],
    'GET /api/projects': () => Response.json([project]),
    'GET /api/projects/p1/threads': [threadFixture],
    'GET /api/threads/t1/entries': o.entries ?? [userEntry('u1', 'hi'), agentEntry('a1', 'hello')],
    'GET /api/threads/t1/operations': o.operations ?? [],
    'POST /api/threads/t1/session': o.session ?? fakeChoices,
    'PUT /api/threads/t1/session/model':
      o.model ??
      (async (r: Request) => {
        const { model } = (await r.json()) as { model: string }
        return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, model } })
      }),
    'POST /api/threads/t1/turns':
      o.start ?? (() => Response.json({ operation_id: 'op1' }, { status: 202 })),
    // The two PATCHes answer what they were asked to become.
    'PATCH /api/threads/t1': async (r: Request) => {
      const { harness } = (await r.json()) as { harness: string }
      return Response.json({ ...threadFixture, harness })
    },
    'PATCH /api/projects/p1': async (r: Request) => {
      const { allowed_modes } = (await r.json()) as { allowed_modes: Record<string, string[]> }
      project = { ...project, allowed_modes }
      return Response.json(project)
    },
    'POST /api/threads/t1/fork':
      o.fork ?? (() => Response.json({ ...threadFixture, id: 't9' }, { status: 201 })),
    'GET /api/threads/t1/context': o.context ?? { categories: null, reason: 'No turn has run yet' },
    'GET /api/projects/p1/workflows': o.plans ?? [],
    'GET /api/workflows/w1': o.plan ?? planFixture(),
    'POST /api/workflows/w1/approve':
      o.approve ??
      (() =>
        Response.json({
          workflow_id: 'w1',
          version: 1,
          revision: 3,
          frozen_at: '2026-09-25T00:00:00Z',
        })),
  }
}
