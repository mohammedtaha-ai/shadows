export type ModelPick = { id: string; contextWindow: number }

export function idError(id: string): string | null {
  if (!id || id.length > 128 || /[\s\p{Cc}]/u.test(id)) return 'Models must be at most 128 printable characters with no spaces.'
  return id === 'default' ? 'Models must not include default: that id is reserved.' : null
}
export function modelPicksError(picked: ModelPick[]): string | null {
  if (picked.length < 1 || picked.length > 20) return 'Models must be 1 to 20 ids.'
  if (new Set(picked.map(model => model.id)).size !== picked.length) return 'Models must be listed once each.'
  for (const model of picked) {
    const error = idError(model.id)
    if (error) return error
    if (!Number.isInteger(model.contextWindow) || model.contextWindow < 1000 || model.contextWindow > 10000000) return 'Context window must be a whole number from 1000 to 10000000.'
  }
  return null
}

