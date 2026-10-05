import { describe, expect, it } from 'vitest'
import { fakeChoices } from '@/test/contract-fixtures'
import { sessionCommand } from './session-commands'

describe('sessionCommand', () => {
  const c = fakeChoices

  it('leaves any other text alone', () => {
    expect(sessionCommand('/compact', c)).toBeNull()
    expect(sessionCommand('/models x', c)).toBeNull()
    expect(sessionCommand('use /model x', c)).toBeNull()
  })

  it('opens the picker when no choice is named', () => {
    expect(sessionCommand('/model', c)).toEqual({ kind: 'open', setting: 'model' })
    expect(sessionCommand('/effort  ', c)).toEqual({ kind: 'open', setting: 'effort' })
  })

  it('names a choice by id, label or a part of either, ignoring case', () => {
    const small = { kind: 'set', setting: 'model', id: 'fake-small' }
    expect(sessionCommand('/model fake-small', c)).toEqual(small)
    expect(sessionCommand('/model SMALL', c)).toEqual(small)
    expect(sessionCommand('/effort max', c)).toEqual({ kind: 'set', setting: 'effort', id: 'max' })
  })

  it('refuses a choice the session does not offer, or no session', () => {
    expect(sessionCommand('/model opus', c)?.kind).toBe('unknown')
    expect(sessionCommand('/model small', null)?.kind).toBe('unknown')
  })
})
