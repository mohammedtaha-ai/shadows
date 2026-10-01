import { describe, expect, it } from 'vitest'
import { choice, fakeChoices } from '@/test/contract-fixtures'
import { afterOptions, initialSettings, modeNote, sendable, withModel } from './turn-settings'

describe('turn settings', () => {
  it('starts from the session current values', () => {
    expect(initialSettings(fakeChoices)).toEqual(fakeChoices.current)
  })

  it('changing model resets an effort it does not offer', () => {
    const small = {
      ...fakeChoices,
      current: { ...fakeChoices.current, model: 'fake-small' },
      efforts: [choice('low'), choice('high')],
    }
    expect(
      withModel(small, { model: 'fake-large', mode: 'acceptEdits', effort: 'max' }, 'fake-small')
        .effort,
    ).toBe('low')
  })

  it('keeps the effort until the session reports the new model, whose efforts are unknown', () => {
    const s = withModel(fakeChoices, fakeChoices.current, 'fake-small')
    expect(s).toEqual({ ...fakeChoices.current, model: 'fake-small' })
  })

  it('a model that offers no effort runs with none', () => {
    const noEffort = { ...fakeChoices, efforts: [], current: { ...fakeChoices.current, model: 'fake-small' } }
    expect(withModel(noEffort, fakeChoices.current, 'fake-small').effort).toBeNull()
  })

  it('a mode the project disallowed is not sendable', () => {
    const c = {
      ...fakeChoices,
      modes: [choice('acceptEdits'), { ...choice('auto'), enabled: false, reason: 'Not allowed in this project' }],
    }
    expect(sendable(c, { ...c.current, mode: 'auto' })).toBe(false)
    expect(initialSettings({ ...c, current: { ...c.current, mode: 'auto' } }).mode).toBe('acceptEdits')
  })

  it('no mode left is not sendable', () => {
    const c = { ...fakeChoices, modes: fakeChoices.modes.map((m) => ({ ...m, enabled: false })) }
    expect(sendable(c, initialSettings(c))).toBe(false)
  })

  it('names the model that moved the mode', () => {
    const next = { ...fakeChoices, current: { model: 'fake-small', mode: 'acceptEdits', effort: 'high' } }
    expect(modeNote({ model: 'fake-small', mode: 'auto', effort: 'high' }, next)).toBe(
      'fake-small does not offer Auto; switched to Accept edits',
    )
    expect(modeNote({ model: 'fake-small', mode: 'acceptEdits', effort: 'high' }, next)).toBeNull()
  })

  it('a picked model the session moves to takes the effort the session reports for it', () => {
    // fake-large ran at max; fake-small, also offering max, was remembered at low.
    const chosen = { model: 'fake-small', mode: 'acceptEdits', effort: 'max' }
    const next = { ...fakeChoices, current: { model: 'fake-small', mode: 'acceptEdits', effort: 'low' } }
    expect(afterOptions(fakeChoices, next, chosen, false).settings.effort).toBe('low')
    // The person's effort on a model the session already holds stays theirs.
    expect(afterOptions(next, { ...next }, chosen, false).settings.effort).toBe('max')
  })

  it('follows a model change the session reports, but not while a send or a turn is in flight', () => {
    const chosen = { model: 'fake-small', mode: 'auto', effort: 'max' }
    const next = {
      ...fakeChoices,
      efforts: [choice('low'), choice('high')],
      current: { model: 'fake-small', mode: 'acceptEdits', effort: 'low' },
    }
    const moved = afterOptions(fakeChoices, next, chosen, false)
    expect(moved.settings).toEqual({ model: 'fake-small', mode: 'acceptEdits', effort: 'low' })
    expect(moved.note).toBe('fake-small does not offer Auto; switched to Accept edits')
    // While a turn starts, the daemon sets the model before the mode: the
    // first report is not the model refusing the mode.
    const busy = afterOptions(fakeChoices, next, chosen, true)
    expect(busy.settings).toEqual({ model: 'fake-small', mode: 'auto', effort: 'low' })
    expect(busy.note).toBeNull()
  })
})
