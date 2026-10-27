// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { usageKeys, type UsageReport, type ProviderUsage } from '@/api/usage'
import { startApp, until, type TestApp, type Answer } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; vi.restoreAllMocks() })
const provider = (slug: string, label: string, input = 600, output = 400): ProviderUsage => ({ provider_slug: slug, provider_name: 'MiniMax', label, requests: 3, tokens_in: input, tokens_out: output, cost: null, currency: null })
const report = (): UsageReport => ({ by_provider: [provider('work', 'Work'), provider('home', 'Home', 300, 200)], by_model: [
  { model: 'm1', provider_slug: 'work', requests: 2, tokens_in: 100, tokens_out: 10 },
  { model: 'm1', provider_slug: 'home', requests: 1, tokens_in: 200, tokens_out: 20 },
  { model: 'm2', provider_slug: 'gone', requests: 1, tokens_in: 0, tokens_out: 0 },
] })
const table = (name: string) => [...document.querySelectorAll('table')].find(element => element.querySelector('caption')?.textContent === name)!
const period = () => document.querySelector<HTMLSelectElement>('select[aria-label="Period"]')!
async function open(usage: Answer = report()) {
  const a = app = await startApp('/?settings=usage', { ...answers(), 'GET /api/aylul/usage': usage })
  return a
}
function select(value: string) {
  const clock = vi.spyOn(Date, 'now').mockReturnValue(Date.parse('2026-10-21T12:00:00.000Z'))
  try { act(() => { period().value = value; period().dispatchEvent(new Event('change', { bubbles: true })) }) }
  finally { clock.mockRestore() }
}
it('Usage opens with Last 7 days', async () => {
  let request!: Request
  const a = await open((incoming: Request) => { request = incoming; return Response.json(report()) })
  await until(() => table('By provider') !== undefined)
  expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('Usage')
  expect(period().value).toBe('7d'); expect(table('By model')).toBeDefined()
  const query = new URL(request.url).searchParams
  expect(new Date(query.get('to')!).getTime() - new Date(query.get('from')!).getTime()).toBe(7 * 86400000)
  expect(query.has('command_id')).toBe(false); expect(await request.text()).toBe('')
  expect(a.calls.filter(call => /^(PUT|POST|DELETE)/.test(call))).toEqual([])
})
it('changing the period fetches its own report', async () => {
  const requests: URL[] = []
  const replies: ((response: Response) => void)[] = []
  const a = await open((request: Request) => {
    requests.push(new URL(request.url))
    if (requests.length === 1) return Response.json(report())
    return new Promise<Response>(resolve => { replies.push(resolve) })
  })
  await until(() => table('By provider') !== undefined)
  select('30d'); await until(() => replies.length === 1)
  select('month'); await until(() => replies.length === 2)
  expect(a.text()).toContain('Loading usage…'); expect(table('By provider')).toBeUndefined()
  const end = '2026-10-21T12:00:00.000Z'
  expect(requests[1].searchParams.get('from')).toBe('2026-09-21T12:00:00.000Z')
  expect(requests[2].searchParams.get('from')).toBe(new Date(2026, 9, 1).toISOString())
  expect(requests[2].searchParams.get('to')).toBe(end)
  await act(async () => replies[1](Response.json({ by_provider: [provider('new', 'Month')], by_model: [] })))
  await until(() => a.text().includes('MiniMax (Month)'))
  await act(async () => replies[0](Response.json({ by_provider: [provider('old', 'Old')], by_model: [] })))
  expect(a.text()).not.toContain('MiniMax (Old)')
  expect(a.queryClient.getQueryCache().findAll({ queryKey: usageKeys.all() })).toHaveLength(3)
})
it('provider snapshots and labelled keys stay distinct', async () => {
  const a = await open(); await until(() => table('By provider') !== undefined)
  expect([...table('By provider').querySelectorAll('tbody tr')].map(row => row.querySelector('th')?.textContent)).toEqual(['MiniMax (Work)', 'MiniMax (Home)'])
  expect(a.calls).not.toContain('GET /api/aylul/providers')
})
it('provider bars compare total tokens', async () => {
  await open(); await until(() => document.querySelector('[role="meter"]') !== null)
  const meters = [...document.querySelectorAll<HTMLElement>('[role="meter"]')]
  expect(meters.map(meter => meter.getAttribute('aria-valuenow'))).toEqual(['1000', '500'])
  expect(meters.map(meter => meter.getAttribute('aria-valuemax'))).toEqual(['1000', '1000'])
  expect(meters.map(meter => (meter.firstElementChild as HTMLElement).style.width)).toEqual(['100%', '50%'])
  app!.unmount(); app = null
  await open({ by_provider: [provider('zero', 'Zero', 0, 0)], by_model: [] })
  await until(() => document.querySelector('[role="meter"]') !== null)
  expect(document.querySelector('[role="meter"]')?.getAttribute('aria-valuemax')).toBe('1')
  expect(document.querySelector<HTMLElement>('[role="meter"] > *')?.style.width).toBe('0%')
})
it('model rows include their provider', async () => {
  await open(); await until(() => table('By model') !== undefined)
  const rows = [...table('By model').querySelectorAll('tbody tr')]
  expect(rows).toHaveLength(3)
  expect(rows[0].textContent).toContain('MiniMax (Work)'); expect(rows[1].textContent).toContain('MiniMax (Home)')
  expect(rows[2].textContent).toContain('gone')
})
it('unknown charges are not zero', async () => {
  await open({ by_provider: [provider('null', 'Null'), { ...provider('zero', 'Zero'), cost: 0, currency: 'USD' },
    { ...provider('usd', 'Usd'), cost: 0.25, currency: 'USD' }, { ...provider('missing', 'Missing'), cost: 1, currency: null },
    { ...provider('eur', 'Eur'), cost: 2, currency: 'EUR' }], by_model: [] })
  await until(() => table('By provider') !== undefined)
  const costs = [...table('By provider').querySelectorAll('tbody tr')].map(row => row.querySelector('[data-usage-cost]')?.textContent)
  expect(costs).toEqual(['–', '0 USD', '0.25 USD', '–', '2 EUR'])
  expect(table('By provider').querySelector('tfoot')).toBeNull()
})
it('empty and failed usage are distinct', async () => {
  let fail = true
  const ranges: string[] = []
  const a = await open((request: Request) => { ranges.push(request.url); return fail ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Usage unavailable.' }, { status: 500 }) : Response.json({ by_provider: [], by_model: [] }) })
  await until(() => a.text().includes('Usage unavailable.'))
  expect(a.text()).not.toContain('No usage in this period.')
  fail = false; act(() => a.button('Retry')!.click())
  await until(() => a.text().includes('No usage in this period.'))
  expect(ranges[1]).toBe(ranges[0])
})
it('a refused period is shown without invented rows', async () => {
  const a = await open(() => Response.json({ code: 'PROFILE_INVALID', message: 'Invalid period.' }, { status: 400 }))
  await until(() => a.text().includes('Invalid period.'))
  expect(table('By provider')).toBeUndefined()
})
