import { invoke } from '@tauri-apps/api/core';
export function hideToTray(): Promise<void> { return invoke('hide_main'); }
