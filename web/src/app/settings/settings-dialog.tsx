import { type ComponentType, useState } from 'react'
import { useSearch } from '@tanstack/react-router'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { type SectionId, visibleSections } from './sections'
import { GeneralSection } from './sections/general'
import { AccountSection } from './sections/account'
import { desktopSettingsSections } from '@/app/desktop'
import { ProvidersSection } from './sections/providers'
import { AgentsSection } from './sections/agents'
import { UsageSection } from './sections/usage'
import { PluginsSection } from './sections/plugins'
import { useCloseSettings, useOpenSettings } from '@/app/navigation'

// A section with a half-filled form reports it, so nothing but its own Save or Cancel leaves it.
const CONTENT: Partial<Record<SectionId, ComponentType<{ onUnsavedChange?: (unsaved: boolean) => void }>>> = {
  general: GeneralSection, account: AccountSection, providers: ProvidersSection, agents: AgentsSection, usage: UsageSection, plugins: PluginsSection,
  ...Object.fromEntries(desktopSettingsSections.map(section => [section.id, section.content])),
}

export function SettingsDialog() {
  const { settings } = useSearch({ from: '__root__' })
  const close = useCloseSettings()
  const open = useOpenSettings()
  const sections = visibleSections()
  const selected = sections.find(section => section.id === settings)
  const Content = selected ? CONTENT[selected.id] : undefined
  const [unsaved, setUnsaved] = useState(false)

  return (
    <Dialog open={settings !== undefined} onOpenChange={value => { if (!value && !unsaved) close() }}>
      <DialogContent
        className="h-[min(620px,calc(100dvh-2rem))] grid-cols-[minmax(140px,220px)_minmax(0,1fr)] gap-0 overflow-hidden border-accent-line/25 bg-popover/95 p-0 backdrop-blur-2xl sm:max-w-[920px]"
        finalFocus={() => document.querySelector<HTMLButtonElement>('[data-settings-trigger]')}
      >
        <nav aria-label="Settings sections" className="overflow-y-auto border-r border-border bg-sidebar/80 px-3 py-5">
          {(['You', 'AI', 'This machine'] as const).map(group => {
            const items = sections.filter(section => section.group === group)
            return items.length > 0 && (
              <div key={group} className="mb-5 space-y-1">
                <h2 className="px-2.5 pb-1 text-2xs font-semibold tracking-wider text-faint-foreground uppercase">{group}</h2>
                {items.map(section => (
                  <button key={section.id} type="button" disabled={unsaved && selected?.id !== section.id} aria-current={selected?.id === section.id ? 'page' : undefined}
                    className={`w-full rounded-xl px-2.5 py-2 text-left text-sm transition-all disabled:opacity-50 ${selected?.id === section.id ? 'pill-active font-medium' : 'text-muted-foreground hover:bg-accent-line/10 hover:text-foreground'}`}
                    onClick={() => open(section.id)}>{section.label}</button>
                ))}
              </div>
            )
          })}
        </nav>
        <div className="min-w-0 overflow-y-auto px-7 py-6">
          <DialogTitle className="mb-6 pr-8 text-base font-semibold tracking-tight text-foreground">{selected?.label ?? 'General'}</DialogTitle>
          {Content && <Content key={selected?.id} onUnsavedChange={setUnsaved} />}
        </div>
      </DialogContent>
    </Dialog>
  )
}
