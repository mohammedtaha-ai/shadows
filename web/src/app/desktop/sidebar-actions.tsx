import { useState } from 'react'
import { Link } from '@tanstack/react-router'
import { capabilities, hideToTray } from '@/desktop'

export function DesktopSidebarActions() {
  const [nativeError, setNativeError] = useState<string | null>(null)
  return (
    <>
      {(capabilities.localCore || capabilities.tray) && <div className="flex gap-3 px-4 py-2 text-xs">
        {capabilities.localCore && <Link to="/aylul/activity">Local activity</Link>}
        {capabilities.tray && <button onClick={() => { void hideToTray().catch(() => setNativeError('Unable to hide the window. Please retry.')) }}>Hide to tray</button>}
      </div>}
      {nativeError && <p role="alert" className="px-4 text-xs">{nativeError}</p>}
    </>
  )
}
