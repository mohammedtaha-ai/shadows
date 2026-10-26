import { isDesktop } from './runtime'

// Read the runtime when a feature asks, so the native boundary has one owner.
export const capabilities = {
  get localCore(): boolean { return isDesktop() },
  get companion(): boolean { return isDesktop() },
  get tray(): boolean { return isDesktop() },
  get accountLogin(): boolean { return isDesktop() },
  get windowPreferences(): boolean { return isDesktop() },
  get hashRouting(): boolean { return isDesktop() },
}
