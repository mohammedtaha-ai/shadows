import type { Provider, ProviderPreset } from '@/api/aylul-profiles'
export type Preset = ProviderPreset
export type PresetGroup = { name: string; presets: Preset[] }

export function presetGroups(presets: Preset[]): PresetGroup[] {
  const groups = new Map<string, PresetGroup>()
  for (const preset of presets) {
    let group = groups.get(preset.name)
    if (!group) { group = { name: preset.name, presets: [] }; groups.set(preset.name, group) }
    group.presets.push(preset)
  }
  return [...groups.values()]
}

export function presetOf(presets: Preset[], provider: Provider): Preset | null {
  return presets.find(preset => preset.id === provider.preset) ?? null
}
export function providerTitle(provider: Provider, presets: Preset[]): string {
  const preset = presetOf(presets, provider)
  const name = preset ? `${preset.name}${preset.region === 'china' ? ' China' : ''}` : provider.name
  return `${name}${provider.label ? ` (${provider.label})` : ''}`
}
