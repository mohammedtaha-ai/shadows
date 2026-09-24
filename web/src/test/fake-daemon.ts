// One job: the route table of a daemon holding one project (`p1`) with one
// conversation (`t1`), for `startApp`. Each answer is the contract fixture
// unless the test overrides it. Test-only.

import type { Operation, Project, ThreadEntry } from '@/api/client'
import type { Answer } from '@/app/test-app'
import {
  agentEntry,
  claudeHarness,
  codexHarness,
  fakeChoices,
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
  /** `POST /api/threads/t1/turns` */
  start?: Answer
  /** `POST /api/threads/t1/fork` */
  fork?: Answer
  /** `GET /api/threads/t1/context` */
  context?: Answer
}

export function answers(o: Overrides = {}): Record<string, Answer> {
  const project = o.project ?? projectFixture
  return {
    'GET /api/harnesses': [claudeHarness, codexHarness],
    'GET /api/projects': [project],
    'GET /api/projects/p1/threads': [threadFixture],
    'GET /api/threads/t1/entries': o.entries ?? [userEntry('u1', 'hi'), agentEntry('a1', 'hello')],
    'GET /api/threads/t1/operations': o.operations ?? [],
    'POST /api/threads/t1/session': o.session ?? fakeChoices,
    'POST /api/threads/t1/turns':
      o.start ?? (() => Response.json({ operation_id: 'op1' }, { status: 202 })),
    // The two PATCHes answer what they were asked to become.
    'PATCH /api/threads/t1': async (r: Request) => {
      const { harness } = (await r.json()) as { harness: string }
      return Response.json({ ...threadFixture, harness })
    },
    'PATCH /api/projects/p1': async (r: Request) => {
      const { allowed_modes } = (await r.json()) as { allowed_modes: Record<string, string[]> }
      return Response.json({ ...project, allowed_modes })
    },
    'POST /api/threads/t1/fork':
      o.fork ?? (() => Response.json({ ...threadFixture, id: 't9' }, { status: 201 })),
    'GET /api/threads/t1/context': o.context ?? { categories: null, reason: 'No turn has run yet' },
  }
}
