import { nativeCall } from './native'

export type DesktopPreferences = { launchAtLogin: boolean; closeToTray: boolean }

function preferences(value: unknown, code: string): DesktopPreferences {
  if (!value || typeof value !== 'object' || !('launchAtLogin' in value) || !('closeToTray' in value) ||
    typeof value.launchAtLogin !== 'boolean' || typeof value.closeToTray !== 'boolean') throw new Error(code)
  return { launchAtLogin: value.launchAtLogin, closeToTray: value.closeToTray }
}
export async function getDesktopPreferences(): Promise<DesktopPreferences> {
  return preferences(await nativeCall('desktop_preferences'), 'desktop_preferences_load_failed')
}
export async function setDesktopPreferences(patch: Partial<DesktopPreferences>): Promise<DesktopPreferences> {
  return preferences(await nativeCall('set_desktop_preferences', { ...patch }), 'desktop_preferences_save_failed')
}
