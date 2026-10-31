// One job: the button that hides or shows the sidebar.

import { PanelLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

export function SidebarToggle({ shown, onToggle, className }: { shown: boolean; onToggle: () => void; className?: string }) {
  const label = shown ? 'Hide sidebar' : 'Show sidebar'
  return <Tooltip><TooltipTrigger render={<Button variant="ghost" size="icon" className={className} aria-label={label} aria-expanded={shown} aria-controls="app-sidebar" onClick={onToggle}><PanelLeft aria-hidden /></Button>} /><TooltipContent>{label} (Ctrl+B)</TooltipContent></Tooltip>
}
