import { describe, expect, it } from 'vitest'
import { attemptFor } from './command-id'

let n = 0
const fresh = () => `id-${++n}`

describe('attemptFor', () => {
  it('reuses the key when the same request is sent again', () => {
    const first = attemptFor(null, { name: 'Demo', directory: 'C:\\work' }, fresh)
    const retry = attemptFor(first, { directory: 'C:\\work', name: 'Demo' }, fresh)
    expect(retry.commandId).toBe(first.commandId)
  })

  it('gives a changed request a fresh key', () => {
    const first = attemptFor(null, { name: 'Demo', directory: 'C:\\work' }, fresh)
    const changed = attemptFor(first, { name: 'Demo', directory: 'C:\\other' }, fresh)
    expect(changed.commandId).not.toBe(first.commandId)
  })

  it('gives a fresh key once the previous attempt is forgotten', () => {
    const first = attemptFor(null, { name: 'Demo' }, fresh)
    expect(attemptFor(null, { name: 'Demo' }, fresh).commandId).not.toBe(first.commandId)
  })
})
