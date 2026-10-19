import React from 'react';
import { getState, type AylulState } from '../platform/aylul';

/** The sidecar's state, refreshed every second while the panel is visible. */
export function useAylulState(intervalMs = 1000) {
  const [state, setState] = React.useState<AylulState | null>(null);
  const refresh = React.useCallback(async () => {
    try { setState(await getState()); } catch { setState(null); }
  }, []);
  React.useEffect(() => {
    void refresh();
    const timer = setInterval(() => { if (!document.hidden) void refresh(); }, intervalMs);
    return () => clearInterval(timer);
  }, [refresh, intervalMs]);
  return { state, refresh };
}
