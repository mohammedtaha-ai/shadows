import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { writeSaved } from './saved-answer'

function held<T>() {
  let release!: (value: T) => void
  const promise = new Promise<T>(resolve => { release = resolve })
  return { promise, release }
}

describe('writeSaved', () => {
  it('a read sent before the save cannot overwrite the saved answer', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const read = held<string>()
    const observer = new QueryObserver(client, { queryKey: ['name'], queryFn: () => read.promise })
    const stop = observer.subscribe(() => {})

    await writeSaved(client, ['name'], 'new')
    read.release('old')
    await new Promise(resolve => setTimeout(resolve, 0))

    expect(client.getQueryData(['name'])).toBe('new')
    stop()
  })

  it('applies an updater to what the cache holds', async () => {
    const client = new QueryClient()
    client.setQueryData(['list'], [1, 2, 3])
    await writeSaved<number[]>(client, ['list'], list => list?.filter(n => n !== 2))
    expect(client.getQueryData(['list'])).toEqual([1, 3])
  })

  it('leaves a read under a longer key fetching', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const read = held<string>()
    const observer = new QueryObserver(client, { queryKey: ['projects', 'p1', 'threads'], queryFn: () => read.promise })
    const stop = observer.subscribe(() => {})

    await writeSaved(client, ['projects'], [])
    read.release('threads')
    await new Promise(resolve => setTimeout(resolve, 0))

    expect(client.getQueryData(['projects', 'p1', 'threads'])).toBe('threads')
    stop()
  })
})
