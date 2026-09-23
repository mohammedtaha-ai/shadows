// One job: what a harness's operational label (a `meta` frame) means to a
// person, or that it means nothing worth a line.

import { Activity, type LucideIcon, Play, RefreshCw, Timer, Wrench } from 'lucide-react'

export interface Shown {
  readonly icon: LucideIcon
  readonly text: string
}

/** Streaming bookkeeping the reply itself already shows. */
const SILENT = new Set([
  'stream_event',
  'message_start',
  'message_delta',
  'message_stop',
  'content_block_start',
  'content_block_stop',
  'ping',
])

export function describeLabel(label: string): Shown | null {
  if (SILENT.has(label)) return null
  if (label === 'system/api_retry') return { icon: RefreshCw, text: 'Retrying the API' }
  if (label.includes('rate_limit')) return { icon: Timer, text: 'Rate limited, waiting' }
  if (label === 'system/init') return { icon: Play, text: 'Session started' }
  if (label === 'user') return { icon: Wrench, text: 'Using a tool' }
  return { icon: Activity, text: label }
}
