import { useEffect, useRef, useState } from 'react'
import { getDesktopPreferences, setDesktopPreferences, type DesktopPreferences } from '@/desktop/preferences'
import { Button } from '@/components/ui/button'

export function DesktopPreferencesControls() {
  const [preferences, setPreferences] = useState<DesktopPreferences | null>(null)
  const [busy, setBusy] = useState(true)
  const [readFailed, setReadFailed] = useState(false)
  const [changeFailed, setChangeFailed] = useState(false)
  const generation = useRef(0)
  const locked = useRef(true)

  async function readActual(owner: number) {
    try {
      const actual = await getDesktopPreferences()
      if (generation.current === owner) { setPreferences(actual); setReadFailed(false) }
    } catch { if (generation.current === owner) setReadFailed(true) }
  }
  useEffect(() => {
    const owner = ++generation.current
    void readActual(owner).finally(() => { if (generation.current === owner) { locked.current = false; setBusy(false) } })
    return () => { generation.current = owner + 1 }
  }, [])
  async function retry() {
    if (locked.current) return
    const owner = generation.current
    locked.current = true; setBusy(true)
    await readActual(owner)
    if (generation.current === owner) { locked.current = false; setBusy(false) }
  }
  async function change(patch: Partial<DesktopPreferences>) {
    if (locked.current || !preferences || readFailed) return
    const owner = generation.current
    locked.current = true; setBusy(true); setChangeFailed(false)
    try {
      const actual = await setDesktopPreferences(patch)
      if (generation.current === owner) { setPreferences(actual); setReadFailed(false) }
    } catch {
      if (generation.current === owner) { setChangeFailed(true); await readActual(owner) }
    } finally { if (generation.current === owner) { locked.current = false; setBusy(false) } }
  }
  const disabled = busy || readFailed || !preferences
  return <section aria-label="Desktop preferences" className="space-y-4 rounded-lg border border-border p-4">
    <h2 className="font-medium">This machine</h2>
    {busy && <p role="status">Loading desktop settings…</p>}
    <label className="flex items-center justify-between gap-3 text-sm">Start AYLUL with Windows
      <input type="checkbox" role="switch" aria-label="Start AYLUL with Windows" disabled={disabled} checked={preferences?.launchAtLogin ?? false} onChange={event => { void change({ launchAtLogin: event.target.checked }) }} />
    </label>
    <label className="flex items-center justify-between gap-3 text-sm">Closing the window hides it to the tray
      <input type="checkbox" role="switch" aria-label="Closing the window hides it to the tray" disabled={disabled} checked={preferences?.closeToTray ?? false} onChange={event => { void change({ closeToTray: event.target.checked }) }} />
    </label>
    {changeFailed && <p role="alert">Could not change this setting.</p>}
    {readFailed && <div className="space-y-3"><p role="alert">Could not load desktop settings.</p><Button type="button" variant="outline" disabled={busy} onClick={() => { void retry() }}>Retry desktop settings</Button></div>}
  </section>
}
