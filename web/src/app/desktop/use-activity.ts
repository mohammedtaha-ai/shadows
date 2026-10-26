import {nativeActivityQuery} from '@/api/native-activity'
import {useQuery} from '@tanstack/react-query'
import {getState} from '@/desktop'

/** Poll the native sidecar only while this mounted panel is visible. */
export function useActivity(intervalMs=1000){
  const query=useQuery(nativeActivityQuery(getState,intervalMs))
  return {state:query.isError?null:query.data??null,refresh:async()=>{await query.refetch()}}
}
