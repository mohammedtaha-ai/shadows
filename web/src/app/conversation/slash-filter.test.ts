import { describe, expect, it } from 'vitest'
import { slashMatches } from './slash-filter'

const c = (name: string) => ({ name, description: '', hint: null })
const list = [c('review'), c('superpowers:brainstorming'), c('init'), c('codex:review'), c('brief')]
const names = (r: ReturnType<typeof slashMatches>) => r?.map((x) => x.name) ?? null

describe('slashMatches', () => {
  it('applies only to a whole text of / and no whitespace', () => {
    expect(names(slashMatches(list, '/'))).toEqual(list.map((x) => x.name))
    expect(slashMatches(list, 'hi /br')).toBeNull()
    expect(slashMatches(list, '/br ')).toBeNull()
    expect(slashMatches(list, '/br\nx')).toBeNull()
    expect(slashMatches(list, '')).toBeNull()
  })

  it('orders name prefix, then the part after the last colon, then contains', () => {
    expect(names(slashMatches(list, '/br'))).toEqual(['brief', 'superpowers:brainstorming'])
    expect(names(slashMatches(list, '/rev'))).toEqual(['review', 'codex:review'])
    expect(names(slashMatches(list, '/storm'))).toEqual(['superpowers:brainstorming'])
  })

  it('ignores case and keeps the adapter order within a group', () => {
    expect(names(slashMatches(list, '/BR'))).toEqual(['brief', 'superpowers:brainstorming'])
  })

  it('applies with no match, listing nothing', () => {
    expect(slashMatches(list, '/zzz')).toEqual([])
  })
})
