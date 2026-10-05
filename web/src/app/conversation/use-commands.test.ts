import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { clearCommands, commandsKey, replaceCommands } from './use-commands'

describe('the thread command cache', () => {
  it('keeps the last list per thread and clears on a harness switch', () => {
    const qc = new QueryClient()
    const list = [{ name: 'compact', description: '', hint: null }]
    replaceCommands(qc, 't1', list)
    expect(qc.getQueryData(commandsKey('t1'))).toEqual(list)
    expect(qc.getQueryData(commandsKey('t2'))).toBeUndefined()
    clearCommands(qc, 't1')
    expect(qc.getQueryData(commandsKey('t1'))).toEqual([])
  })
})
