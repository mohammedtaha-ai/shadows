import { type DesktopSectionId, visibleDesktopSettingsSections } from '@/app/desktop'

export type SectionId = 'account' | 'general' | 'providers' | 'agents' | 'plugins' | 'usage' | DesktopSectionId

export const SECTIONS: { id: SectionId; label: string; group: 'You' | 'AI' }[] = [
  { id: 'account', label: 'Account', group: 'You' },
  { id: 'general', label: 'General', group: 'You' },
  { id: 'providers', label: 'Providers', group: 'AI' },
  { id: 'agents', label: 'Agents', group: 'AI' },
  { id: 'plugins', label: 'Plugins & Skills', group: 'AI' },
  { id: 'usage', label: 'Usage', group: 'AI' },
]

export function visibleSections() {
  return [...SECTIONS, ...visibleDesktopSettingsSections()]
}

export function readSettingsSearch(search: Record<string, unknown>): { settings?: SectionId; agent?: string } {
  const settings = search.settings === undefined ? undefined :
    visibleSections().find(section => section.id === search.settings)?.id ?? 'general'
  return { settings, agent: settings === 'agents' && typeof search.agent === 'string' ? search.agent : undefined }
}
