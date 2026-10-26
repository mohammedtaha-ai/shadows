import type { ComponentType } from 'react'
import { useSearch } from '@tanstack/react-router'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { type SectionId, visibleSections } from './sections'
import { GeneralSection } from './sections/general'
import { AccountSection } from './sections/account'
import { CharacterSection } from './sections/character'
import { CoreSection } from './sections/core'
import { ProvidersSection } from './sections/providers'
import { AgentsSection } from './sections/agents'
import { UsageSection } from './sections/usage'
import { PluginsSection } from './sections/plugins'
import { useCloseSettings, useOpenSettings } from '@/app/navigation'

const CONTENT: Partial<Record<SectionId, ComponentType>> = { general: GeneralSection, account: AccountSection, character: CharacterSection, core: CoreSection, providers: ProvidersSection, agents: AgentsSection, usage: UsageSection, plugins: PluginsSection }

export function SettingsDialog() {
  const { settings } = useSearch({ from: '__root__' })
  const close = useCloseSettings()
  const open = useOpenSettings()
  const sections = visibleSections()
  const selected = sections.find(section => section.id === settings)
  const Content = selected ? CONTENT[selected.id] : undefined

  return (
    <Dialog open={settings !== undefined} onOpenChange={value => { if (!value) close() }}>
      <DialogContent
        className="h-[min(590px,calc(100dvh-2rem))] grid-cols-[minmax(130px,210px)_minmax(0,1fr)] gap-0 overflow-hidden p-0 sm:max-w-[900px]"
        finalFocus={() => document.querySelector<HTMLButtonElement>('[data-settings-trigger]')}
      >
        <nav aria-label="Settings sections" className="overflow-y-auto border-r border-border bg-muted/30 px-3 py-5">
          {(['You', 'AI', 'This machine'] as const).map(group => {
            const items = sections.filter(section => section.group === group)
            return items.length > 0 && (
              <div key={group} className="mb-5 space-y-1">
                <h2 className="px-2 pb-1 text-xs text-faint-foreground">{group}</h2>
                {items.map(section => (
                  <button key={section.id} type="button" aria-current={selected?.id === section.id ? 'page' : undefined}
                    className={`w-full rounded-md px-2 py-2 text-left text-sm hover:bg-secondary ${selected?.id === section.id ? 'bg-secondary font-medium' : ''}`}
                    onClick={() => open(section.id)}>{section.label}</button>
                ))}
              </div>
            )
          })}
        </nav>
        <div className="min-w-0 overflow-y-auto px-6 py-6">
          <DialogTitle className="mb-6 pr-8">{selected?.label ?? 'General'}</DialogTitle>
          {Content && <Content key={selected?.id} />}
        </div>
      </DialogContent>
    </Dialog>
  )
}
