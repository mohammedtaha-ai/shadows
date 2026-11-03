import type { ComponentProps, ReactNode } from 'react'
import { ArrowUp } from 'lucide-react'
import { Button } from '@/components/ui/button'

export function ComposerFrame({ children, ...events }: ComponentProps<'div'>) {
  return (
    <div className="border-t border-border/70 bg-background/90 px-6 pt-3 pb-4 backdrop-blur-xl">
      <div {...events} className="mx-auto max-w-3xl space-y-2.5" data-composer-frame>
        {children}
      </div>
    </div>
  )
}

export function ComposerInput({ input, overlay, action }: {
  input: ComponentProps<'textarea'>
  overlay?: ReactNode
  action: ReactNode
}) {
  return (
    <div data-composer-input className="surface-glow flex items-end gap-2.5 rounded-2xl p-3 transition-all duration-150 focus-within:border-accent-line/80 focus-within:ring-2 focus-within:ring-accent-line/30">
      <div className="relative min-w-0 flex-1">
        <textarea {...input} rows={2}
          className="max-h-48 min-h-10 w-full resize-none bg-transparent px-2 py-1 text-sm text-foreground outline-none placeholder:text-faint-foreground" />
        {overlay}
      </div>
      {action}
    </div>
  )
}

export function ComposerSendButton({ label, disabled, onClick, type = 'button' }: {
  label: string
  disabled: boolean
  onClick?: () => void
  type?: 'button' | 'submit'
}) {
  return (
    <Button type={type} onClick={onClick} disabled={disabled} size="icon" aria-label={label}
      className="rounded-xl bg-gradient-to-br from-accent-line to-accent text-primary-foreground transition-all hover:brightness-110 active:scale-95 disabled:opacity-40">
      <ArrowUp aria-hidden />
    </Button>
  )
}
