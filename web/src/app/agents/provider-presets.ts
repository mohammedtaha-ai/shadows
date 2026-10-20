export type Preset = { id: string; name: string; region: 'international' | 'china' | null; baseUrl: string }
export type PresetGroup = { name: string; presets: Preset[] }

export const PRESETS: Preset[] = [
  { id: 'minimax', name: 'MiniMax', region: 'international', baseUrl: 'https://api.minimax.io/anthropic' },
  { id: 'minimax-china', name: 'MiniMax', region: 'china', baseUrl: 'https://api.minimaxi.com/anthropic' },
  { id: 'anthropic', name: 'Anthropic', region: null, baseUrl: 'https://api.anthropic.com' },
  { id: 'deepseek', name: 'DeepSeek', region: null, baseUrl: 'https://api.deepseek.com/anthropic' },
  { id: 'openrouter', name: 'OpenRouter', region: null, baseUrl: 'https://openrouter.ai/api' },
  { id: 'moonshot', name: 'Moonshot (Kimi)', region: 'international', baseUrl: 'https://api.moonshot.ai/anthropic' },
  { id: 'moonshot-china', name: 'Moonshot (Kimi)', region: 'china', baseUrl: 'https://api.moonshot.cn/anthropic' },
]

export function presetGroups(presets: Preset[]): PresetGroup[] {
  const groups = new Map<string, PresetGroup>()
  for (const preset of presets) {
    let group = groups.get(preset.name)
    if (!group) { group = { name: preset.name, presets: [] }; groups.set(preset.name, group) }
    group.presets.push(preset)
  }
  return [...groups.values()]
}

export function presetFor(baseUrl: string): Preset | null {
  return PRESETS.find(preset => preset.baseUrl === baseUrl) ?? null
}
