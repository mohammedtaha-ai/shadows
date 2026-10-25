import { expect, it } from 'vitest'
import { directoryKeys, dirsQuery } from './directories'
it('directory reads and creation refresh use identical parent keys', () => {
  expect(dirsQuery(null).queryKey).toEqual(['fs', 'dirs', null])
  expect(dirsQuery('E:/project').queryKey).toEqual(['fs', 'dirs', 'E:/project'])
  expect(directoryKeys.children('E:/project')).toEqual(dirsQuery('E:/project').queryKey)
})
