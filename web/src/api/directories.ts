import { queryOptions } from '@tanstack/react-query'
import { listDirs } from './client'
export const directoryKeys = { children: (path: string | null) => ['fs', 'dirs', path] }


/** One directory's subdirectories; `null` lists the roots. Not retried: a
 * path that does not exist is an answer, shown as it is. */
export function dirsQuery(path: string | null) {
  return queryOptions({
    queryKey: directoryKeys.children(path),
    queryFn: () => listDirs(path),
    retry: false,
  })
}
