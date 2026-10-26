// These exports must be safe before native readiness fixes the API origin.
export { bootApplication, isDesktop } from './runtime'
export { prepareDesktop, externalLinks } from './bootstrap'
export { connectionBridge, connectionConfig, saveConnection, retryConnection, type LaunchConfig } from './native'
export { type ConnectionView } from './runtime'
