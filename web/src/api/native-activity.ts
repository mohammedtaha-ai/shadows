import { queryOptions } from '@tanstack/react-query'
export const nativeActivityKeys = { state: () => ['desktop-local-activity'] }
// The native bridge supplies transport; cache ownership must not depend on desktop code.
export function nativeActivityQuery<T>(read: () => Promise<T>, intervalMs: number) {
  return queryOptions({ queryKey: nativeActivityKeys.state(), queryFn: read, retry: false,
    refetchInterval: intervalMs, refetchIntervalInBackground: false })
}
