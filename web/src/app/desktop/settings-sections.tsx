/* eslint-disable react/only-export-components -- Section metadata stays beside its content so settings consumes one desktop-owned definition. */
import type { ComponentType } from 'react'
import { capabilities } from '@/desktop'
import { CompanionControls } from './companion-controls'
import { CompanionSettings } from './companion-settings'
import { ConnectionSettings } from './connection-settings'

export type DesktopSectionId = 'character' | 'core'

export function CharacterSection() {
  return <section className="space-y-5"><CompanionControls /><CompanionSettings /></section>
}

export function CoreSection() {
  return <ConnectionSettings />
}

export const desktopSettingsSections: {
  id: DesktopSectionId
  label: string
  group: 'This machine'
  capability: 'companion' | 'localCore'
  content: ComponentType
}[] = [
  { id: 'character', label: 'AYLUL character', group: 'This machine', capability: 'companion', content: CharacterSection },
  { id: 'core', label: 'Local Core', group: 'This machine', capability: 'localCore', content: CoreSection },
]

export function visibleDesktopSettingsSections() {
  return desktopSettingsSections.filter(section => capabilities[section.capability])
}
