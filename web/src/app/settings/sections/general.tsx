import { ActiveProjects } from '../active-projects'
import { GeneralPreferences } from '../general-preferences'
import { DesktopPreferencesControls } from '../desktop-preferences'
import { capabilities } from '@/desktop'

export function GeneralSection() {
  return (
    <div className="space-y-5"><section className="space-y-4 rounded-lg border border-border p-4" aria-label="Projects">
      <h2 className="text-sm font-medium">Projects</h2>
      <ActiveProjects />
    </section><GeneralPreferences />{capabilities.windowPreferences && <DesktopPreferencesControls />}</div>
  )
}
