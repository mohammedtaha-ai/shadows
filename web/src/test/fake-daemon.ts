// One job: the route table of a daemon holding one project (`p1`) with one
// conversation (`t1`), a ready code index and no code link, and no plan
// (`w1`), instructions or grant unless a test gives it one, for `startApp`.
// Each answer is the contract fixture unless the test overrides it. Test-only.

import type {
  Grant,
  InstructionsVersion,
  Operation,
  Plan,
  PlanListing,
  Project,
  ThreadEntry,
} from '@/api/client'
import type { Answer } from '@/app/test-app'
import {
  agentEntry,
  claudeHarness,
  codeStatusFixture,
  codexHarness,
  fakeChoices,
  grantFixture,
  planFixture,
  planVersionsFixture,
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
  /** `PUT /api/threads/t1/session/effort`; by default the session takes the
   * effort and answers `fakeChoices` holding it. */
  effort?: Answer
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
  /** The project's saved instructions; none by default. */
  instructions?: InstructionsVersion
  /** The project's grants, newest first; none by default. */
  grants?: Grant[]
  /** `POST /api/projects/p1/mcp-grants`; by default issues grant `g9` with
   * its token and command. */
  issue?: Answer
}

export function answers(o: Overrides = {}): Record<string, Answer> {
  // Kept as the daemon keeps it: a refetch after a PATCH reads what the PATCH
  // saved, as it does against the real daemon.
  let project = o.project ?? projectFixture
  let instructions = o.instructions ?? null
  let grants = o.grants ?? []
  const connect = 'claude mcp add --transport http shadows http://127.0.0.1:4318/mcp'
  return {
    'GET /api/harnesses': [claudeHarness, codexHarness],
    'GET /api/projects': () => Response.json([project]),
    'GET /api/projects/p1/threads': [threadFixture],
    'GET /api/projects/p1/agreements': [],
    'GET /api/threads/t1': threadFixture,
    'GET /api/threads/t1/entries': o.entries ?? [userEntry('u1', 'hi'), agentEntry('a1', 'hello')],
    'GET /api/threads/t1/operations': o.operations ?? [],
    'POST /api/threads/t1/session': o.session ?? fakeChoices,
    'PUT /api/threads/t1/session/model':
      o.model ??
      (async (r: Request) => {
        const { model } = (await r.json()) as { model: string }
        return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, model } })
      }),
    'PUT /api/threads/t1/session/effort':
      o.effort ??
      (async (r: Request) => {
        const { effort } = (await r.json()) as { effort: string }
        return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, effort } })
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
    'GET /api/projects/p1/workflows': (request: Request) => {
      const url = new URL(request.url)
      const all = o.plans ?? []
      if (url.searchParams.get('archived') === 'true') {
        return Response.json(all)
      }
      return Response.json(all.filter((p) => p.plan_state !== 'Archived'))
    },
    'GET /api/workflows/w1': o.plan ?? planFixture(),
    'GET /api/plans/plan1': () => {
      const basePlan = typeof o.plan === 'function' ? planFixture() : (o.plan ?? planFixture())
      return Response.json(planVersionsFixture(basePlan as Plan))
    },
    'POST /api/plans/plan1/archive': () => {
      const basePlan = typeof o.plan === 'function' ? planFixture() : (o.plan ?? planFixture())
      return Response.json(planVersionsFixture({ ...(basePlan as Plan), plan_state: 'Archived' }))
    },
    'POST /api/plans/plan1/unarchive': () => {
      const basePlan = typeof o.plan === 'function' ? planFixture() : (o.plan ?? planFixture())
      return Response.json(planVersionsFixture({ ...(basePlan as Plan), plan_state: 'Active' }))
    },
    'POST /api/workflows/w1/approve':
      o.approve ??
      (() =>
        Response.json({
          workflow_id: 'w1',
          version: 1,
          revision: 3,
          frozen_at: '2026-09-25T00:00:00Z',
        })),
    'GET /api/projects/p1/planner-instructions': () => Response.json(instructions),
    'PUT /api/projects/p1/planner-instructions': async (r: Request) => {
      const { body } = (await r.json()) as { body: string }
      const number = (instructions?.number ?? 0) + 1
      instructions = { body, number, created_at: '2026-09-25T09:30:00Z' }
      return Response.json(instructions)
    },
    'GET /api/projects/p1/mcp-grants': () => Response.json(grants),
    'POST /api/projects/p1/mcp-grants':
      o.issue ??
      (() => {
        const grant = grantFixture('g9')
        grants = [grant, ...grants]
        return Response.json({
          grant,
          token: 'tok-9',
          command: `${connect} --header "Authorization: Bearer tok-9"`,
        })
      }),
    'GET /api/projects/p1/code/status': codeStatusFixture(project.slug),
    'GET /api/projects/p1/code/links': [],
    'GET /api/code/settings': { active_limit: 5 },
    // Any grant the test gave the project can be revoked.
    ...Object.fromEntries(
      grants.map((g) => [
        `DELETE /api/mcp-grants/${g.id}`,
        () => {
          const revoked = { ...g, revoked_at: '2026-09-25T10:00:00Z' }
          grants = grants.map((x) => (x.id === g.id ? revoked : x))
          return Response.json(revoked)
        },
      ]),
    ),
  }
}
