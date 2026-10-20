// One job: the global Settings page (§13.11) — the daemon's settings that
// belong to no one project, one section each. A new setting is one more
// section here.

import { ActiveProjects } from './active-projects'

export function SettingsPage() {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-2xl space-y-8 px-6 py-8">
        <header className="space-y-1">
          <h1 className="text-base font-medium">Settings</h1>
          <p className="text-xs text-faint-foreground">For every project on this machine.</p>
        </header>
        <ActiveProjects />
      </div>
    </div>
  )
}
