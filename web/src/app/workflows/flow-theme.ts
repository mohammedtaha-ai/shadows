import type { CSSProperties } from 'react'

/** React Flow's own colours, taken from Shadows' theme tokens. Every canvas uses them. */
export const THEME = {
  '--xy-background-color': 'var(--background)',
  '--xy-edge-label-background-color': 'var(--background)',
  '--xy-minimap-background-color': 'var(--sidebar)',
  '--xy-minimap-node-background-color': 'var(--muted)',
  '--xy-minimap-mask-background-color': 'color-mix(in oklab, var(--background) 65%, transparent)',
  '--xy-controls-button-background-color': 'var(--card)',
  '--xy-controls-button-background-color-hover': 'var(--muted)',
  '--xy-controls-button-color': 'var(--muted-foreground)',
  '--xy-controls-button-color-hover': 'var(--foreground)',
  '--xy-controls-button-border-color': 'var(--border)',
  '--xy-attribution-background-color': 'transparent',
} as CSSProperties
