import { queryOptions } from '@tanstack/react-query'
import { listHarnesses } from './client'
export const harnessKeys = { list: () => ['harnesses'] }
// Harness capabilities can change, so reads retain normal focus refetching.
export const harnessesQuery = queryOptions({ queryKey: harnessKeys.list(), queryFn: listHarnesses })
