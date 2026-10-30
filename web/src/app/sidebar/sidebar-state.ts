import { useCallback, useEffect, useState } from 'react'
import { useRouterState } from '@tanstack/react-router'

const key = 'shadows.sidebar.collapsed'
const query = '(max-width: 767px)'
function readStored() { try { return localStorage.getItem(key) !== 'true' } catch { return true } }
function writeStored(shown: boolean) { try { localStorage.setItem(key, String(!shown)) } catch { /* Storage is optional. */ } }
function isNarrow() { return typeof window.matchMedia === 'function' && window.matchMedia(query).matches }

export function useSidebarState() {
  const [narrow, setNarrow] = useState(isNarrow)
  const [shown, setShown] = useState(() => !isNarrow() && readStored())
  const pathname = useRouterState({ select: state => state.location.pathname })
  const [previousPath, setPreviousPath] = useState(pathname)
  if (pathname !== previousPath) {
    setPreviousPath(pathname)
    if (narrow) setShown(false)
  }
  const hide = useCallback(() => setShown(false), [])
  const toggle = useCallback(() => setShown(current => {
    const next = !current
    if (!narrow) writeStored(next)
    return next
  }), [narrow])
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return
    const media = window.matchMedia(query)
    const changed = (event: MediaQueryListEvent) => { setNarrow(event.matches); setShown(!event.matches && readStored()) }
    media.addEventListener('change', changed)
    return () => media.removeEventListener('change', changed)
  }, [])
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'b') { event.preventDefault(); toggle() }
      else if (event.key === 'Escape' && narrow && shown) { hide(); requestAnimationFrame(() => document.querySelector<HTMLButtonElement>('main [aria-label="Show sidebar"]')?.focus()) }
    }
    window.addEventListener('keydown', keydown)
    return () => window.removeEventListener('keydown', keydown)
  }, [toggle, narrow, shown, hide])
  return { shown, narrow, toggle, hide }
}
