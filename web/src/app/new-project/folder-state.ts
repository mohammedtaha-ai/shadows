// One job: where the new-project folder browser is, and which folder the
// dialog's Folder field names.
//
// The field and the browser are one choice seen two ways: browsing fills the
// field, and typing a path then pressing Enter browses to it. Typing alone
// does not browse, so a half-typed path never fires a listing per keystroke.

export interface FolderState {
  /** The directory being listed; `null` lists the roots (drives). */
  readonly at: string | null
  /** The Folder field's text: the folder the project will own. */
  readonly input: string
}

export type FolderAction =
  /** A subdirectory was clicked, or "New folder here" just created one. */
  | { type: 'enter'; path: string }
  /** The "up" row: the listing's parent, `null` above a root. */
  | { type: 'up'; parent: string | null }
  /** Typing or pasting into the Folder field. */
  | { type: 'type'; text: string }
  /** Enter in the Folder field: list what it says, or the roots if nothing. */
  | { type: 'go' }

export const initialFolderState: FolderState = { at: null, input: '' }

export function folderReducer(state: FolderState, action: FolderAction): FolderState {
  switch (action.type) {
    case 'enter':
      return { at: action.path, input: action.path }
    case 'up':
      return { at: action.parent, input: action.parent ?? '' }
    case 'type':
      return { ...state, input: action.text }
    case 'go': {
      const path = state.input.trim()
      return path === '' ? { at: null, input: '' } : { at: path, input: path }
    }
  }
}

/** The folder the dialog would create the project in, if any. */
export function chosenFolder(state: FolderState): string | null {
  const path = state.input.trim()
  return path === '' ? null : path
}
