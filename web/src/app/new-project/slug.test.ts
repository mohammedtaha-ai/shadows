import { describe, expect, it } from 'vitest'
import { slugify } from './slug'

describe('slugify', () => {
  it('lowercases and joins words with single dashes', () => {
    expect(slugify('My Planner Project')).toBe('my-planner-project')
    expect(slugify('  Hello,  World!! ')).toBe('hello-world')
  })

  it('keeps digits and drops accents, not their letters', () => {
    expect(slugify('Café Déjà Vu 2')).toBe('cafe-deja-vu-2')
  })

  it('is empty for a name with nothing to keep', () => {
    expect(slugify(' — ')).toBe('')
  })
})
