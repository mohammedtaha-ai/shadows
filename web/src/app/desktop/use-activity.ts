import {useQuery} from '@tanstack/react-query'
import {getState} from '../../desktop/activity-api'

/** Poll the native sidecar only while this mounted panel is visible. */
export function useActivity(intervalMs=1000){
  const query=useQuery({
    queryKey:['desktop-local-activity'],queryFn:getState,retry:false,
    refetchInterval:intervalMs,refetchIntervalInBackground:false,
  })
  return {state:query.isError?null:query.data??null,refresh:async()=>{await query.refetch()}}
}
