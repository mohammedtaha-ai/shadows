// One job: tell the Settings dialog whether the open provider form holds
// edits that closing the dialog would lose.

let dirty = false

export const setProviderDirty = (value: boolean) => { dirty = value }
export const providerFormIsDirty = () => dirty
