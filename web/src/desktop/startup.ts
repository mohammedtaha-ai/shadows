// These exports must be safe before native readiness fixes the API origin.
export { bootApplication, isDesktop } from './runtime'
export { connectionBridge, connectionConfig, saveConnection, retryConnection, type LaunchConfig } from './native'
export { type ConnectionView } from './runtime'
// The native gate (and react-dom with it) stays out of the page's first chunk;
// main.tsx loads it only on the desktop.
export const loadNativeGate = () => import('./bootstrap')
