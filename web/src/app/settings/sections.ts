import { isDesktop } from '@/desktop/runtime'

export type SectionId = 'account' | 'general' | 'providers' | 'agents' | 'plugins' | 'usage' | 'character' | 'core'

export const SECTIONS: { id: SectionId; label: string; group: 'You' | 'AI' | 'This machine'; desktopOnly: boolean }[] = [
  { id: 'account', label: 'Account', group: 'You', desktopOnly: false },
  { id: 'general', label: 'General', group: 'You', desktopOnly: false },
  { id: 'providers', label: 'Providers', group: 'AI', desktopOnly: false },
  { id: 'agents', label: 'Agents', group: 'AI', desktopOnly: false },
  { id: 'plugins', label: 'Plugins & Skills', group: 'AI', desktopOnly: false },
  { id: 'usage', label: 'Usage', group: 'AI', desktopOnly: false },
  { id: 'character', label: 'AYLUL character', group: 'This machine', desktopOnly: true },
  { id: 'core', label: 'Local Core', group: 'This machine', desktopOnly: true },
]

export function visibleSections() {
  return SECTIONS.filter(section => !section.desktopOnly || isDesktop())
}

export function readSettingsSearch(search: Record<string, unknown>): { settings?: SectionId; agent?: string } {
  const settings = search.settings === undefined ? undefined :
    visibleSections().find(section => section.id === search.settings)?.id ?? 'general'
  return { settings, agent: settings === 'agents' && typeof search.agent === 'string' ? search.agent : undefined }
}
