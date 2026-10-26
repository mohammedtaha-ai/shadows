// The request each Phase B call sends, read off a stubbed `fetch`: the route,
// and a body whose command id is always the caller's.

import { afterEach, describe, expect, it, vi } from 'vitest'
import { fakeChoices, threadFixture } from '@/test/contract-fixtures'
import { setProjectModes } from './projects'
import { openSession } from './sessions'
import { attachmentUrl, uploadAttachment, forkThread, setThreadHarness, startTurn } from './threads'

afterEach(() => vi.unstubAllGlobals())

/** Stubs `fetch` to answer `answer`, recording each request. */
function record(answer: () => Response) {
  const calls: string[] = []
  const bodies: unknown[] = []
  vi.stubGlobal('fetch', async (r: Request) => {
    calls.push(`${r.method} ${new URL(r.url).pathname}`)
    const text = await r.text()
    if (text !== '') bodies.push(JSON.parse(text))
    return answer()
  })
  return { calls, bodies }
}

describe('client', () => {
  it("startTurn sends the settings, the caller's command id and its tab", async () => {
    const { bodies } = record(() => Response.json({ operation_id: 'op1' }, { status: 202 }))
    const settings = { model: 'fake-small', mode: 'acceptEdits', effort: 'high' }
    const id = await startTurn('t1', 'cmd-1', 'hi', settings, { clientTab: 'tab-1' })
    const focus = { workflow_id: 'w1', task_id: 'task-4', revision: 3 }
    await startTurn('t1', 'cmd-2', 'change this', settings, { focus, clientTab: 'tab-1' })
    expect(id).toBe('op1')
    expect(bodies).toEqual([
      {
        command_id: 'cmd-1',
        prompt: 'hi',
        model: 'fake-small',
        mode: 'acceptEdits',
        effort: 'high',
        focus: null,
        client_tab: 'tab-1',
      },
      { command_id: 'cmd-2', prompt: 'change this', ...settings, focus, client_tab: 'tab-1' },
    ])
  })

  it('openSession posts to the thread session route', async () => {
    const { calls } = record(() => Response.json(fakeChoices))
    await openSession('t1')
    expect(calls).toEqual(['POST /api/threads/t1/session'])
  })

  it('changes a thread, a project and forks, each with the given command id', async () => {
    const { calls, bodies } = record(() => Response.json(threadFixture))
    await setThreadHarness('t1', 'cmd-h', 'codex')
    await setProjectModes('p1', 'cmd-m', { 'claude-code': ['acceptEdits'] })
    await forkThread('t1', 'cmd-f', 'e9')
    expect(calls).toEqual([
      'PATCH /api/threads/t1',
      'PATCH /api/projects/p1',
      'POST /api/threads/t1/fork',
    ])
    expect(bodies).toEqual([
      { command_id: 'cmd-h', harness: 'codex' },
      { command_id: 'cmd-m', allowed_modes: { 'claude-code': ['acceptEdits'] } },
      { command_id: 'cmd-f', at_entry_id: 'e9' },
    ])
  })
})

it('uploadAttachment sends multipart and attachmentUrl encodes byte-route ids', async () => {
  const file = new File(['image bytes'], 'screen.png', { type: 'image/png' })
  vi.stubGlobal('fetch', async (request: Request) => {
    expect(request.method).toBe('POST')
    expect(new URL(request.url).pathname).toBe('/api/threads/t1/attachments')
    expect(request.headers.get('content-type')).toContain('multipart/form-data; boundary=')
    const body = await request.formData()
    expect(body.get('command_id')).toBe('upload-1')
    const uploaded = body.get('file') as File
    expect(uploaded.name).toBe('screen.png')
    expect(uploaded.type).toBe('image/png')
    expect(await uploaded.text()).toBe('image bytes')
    return Response.json({ id: 'image-1', media_type: uploaded.type, byte_len: uploaded.size, name: uploaded.name })
  })
  expect(await uploadAttachment('t1', 'upload-1', file)).toMatchObject({ id: 'image-1', name: 'screen.png' })
  expect(attachmentUrl('thread/one', 'image two')).toMatch(/\/api\/threads\/thread%2Fone\/attachments\/image%20two$/)
})
