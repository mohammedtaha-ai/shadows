import { GeneralPreferences } from '../general-preferences'
import { DesktopPreferencesControls } from '../desktop-preferences'
import { capabilities } from '@/desktop'

export function GeneralSection() {
  return (
    <div className="space-y-5"><GeneralPreferences />{capabilities.windowPreferences && <DesktopPreferencesControls />}</div>
  )
}
