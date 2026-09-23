import { describe, expect, it } from 'vitest'
import { chosenFolder, folderReducer, initialFolderState } from './folder-state'

describe('folderReducer', () => {
  it('starts at the roots with no folder chosen', () => {
    expect(initialFolderState.at).toBeNull()
    expect(chosenFolder(initialFolderState)).toBeNull()
  })

  it('entering a folder lists it and chooses it', () => {
    const state = folderReducer(initialFolderState, { type: 'enter', path: 'C:\\work' })
    expect(state).toEqual({ at: 'C:\\work', input: 'C:\\work' })
  })

  it('going up lists the parent, and above a root lists the roots', () => {
    const inside = folderReducer(initialFolderState, { type: 'enter', path: 'C:\\work\\a' })
    const parent = folderReducer(inside, { type: 'up', parent: 'C:\\work' })
    expect(parent).toEqual({ at: 'C:\\work', input: 'C:\\work' })
    expect(folderReducer(parent, { type: 'up', parent: null })).toEqual(initialFolderState)
  })

  it('typing chooses without browsing; Enter browses to what was typed', () => {
    const inside = folderReducer(initialFolderState, { type: 'enter', path: 'C:\\work' })
    const typed = folderReducer(inside, { type: 'type', text: '  D:\\src ' })
    expect(typed.at).toBe('C:\\work')
    expect(chosenFolder(typed)).toBe('D:\\src')
    expect(folderReducer(typed, { type: 'go' })).toEqual({ at: 'D:\\src', input: 'D:\\src' })
    expect(folderReducer({ at: 'D:\\src', input: ' ' }, { type: 'go' })).toEqual(initialFolderState)
  })
})
