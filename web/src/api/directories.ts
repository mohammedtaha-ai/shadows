import { client, unwrap } from './transport'
import type { components } from './schema'
import { queryOptions } from '@tanstack/react-query'

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


export type DirectoryListing = components['schemas']['DirectoryListing']

export type DirectoryEntry = components['schemas']['DirectoryEntry']

/** One directory's subdirectories, or with no path the roots (drives). */
export function listDirs(path: string | null): Promise<DirectoryListing> {
  return unwrap(
    client.GET('/api/fs/dirs', { params: { query: path === null ? {} : { path } } }),
  )
}

/** Creates one directory named `name` inside `parent`. */
export function createDir(parent: string, name: string): Promise<DirectoryEntry> {
  return unwrap(client.POST('/api/fs/dirs', { body: { parent, name } }))
}
