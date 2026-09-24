// One job: the flat row under the message box (spec §12.11) — on the left
// `+`, mode and folder; on the right model, effort and the context ring —
// every menu built from what the harness session offers.

import { ChevronDown, Folder, Plus } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Choice, TurnSettings } from '@/api/client'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ErrorLine } from '../error-line'
import { effortsKnown, labelOf, withModel } from './turn-settings'
import type { SessionView } from './use-session'

export function ComposerBar({
  harnessLabel,
  session,
  settings,
  onSettings,
  changingModel,
  directory,
  note,
  ring,
}: {
  harnessLabel: string
  session: SessionView
  /** `null` until the session has answered. */
  settings: TurnSettings | null
  onSettings: (next: TurnSettings) => void
  /** The session is being set to the picked model (spec §12.7). */
  changingModel: boolean
  directory: string | null | undefined
  /** Why the settings moved on their own, if they did. */
  note: string | null
  ring?: ReactNode
}) {
  const choices = session.state === 'ready' ? session.choices : null
  const noMode = choices !== null && !choices.modes.some((m) => m.enabled)

  return (
    <div className="space-y-1.5">
      <div className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground">
        <Button
          variant="ghost"
          size="icon-xs"
          disabled
          focusableWhenDisabled
          title="Attachments come later"
          aria-label="Attach"
        >
          <Plus />
        </Button>
        {session.state === 'connecting' && (
          <span className="px-1.5">Connecting to {harnessLabel}…</span>
        )}
        {session.state === 'failed' && (
          <span className="flex min-w-0 items-center gap-2 px-1.5">
            <ErrorLine error={session.error} />
            <Button variant="outline" size="xs" onClick={session.retry}>
              Retry
            </Button>
          </span>
        )}
        {choices !== null && settings !== null && (
          <Setting
            title="Mode"
            options={choices.modes}
            value={settings.mode}
            onChange={(mode) => onSettings({ ...settings, mode })}
          />
        )}
        {directory != null && (
          <span className="flex min-w-0 items-center gap-1 px-1.5" title={directory}>
            <Folder aria-hidden className="size-3.5 shrink-0" />
            <span className="truncate font-mono text-[11px] text-faint-foreground">
              <span className="sr-only">Runs in </span>
              {directory}
            </span>
          </span>
        )}
        <span className="ml-auto flex shrink-0 items-center gap-1">
          {choices !== null && settings !== null && (
            <>
              <Setting
                title="Model"
                options={choices.models}
                value={settings.model}
                disabled={changingModel}
                onChange={(model) => onSettings(withModel(choices, settings, model))}
              />
              {/* Efforts belong to the model the session holds: until it
                  answers the chosen one they are unknown, and the menu keeps
                  the effort, disabled. A model that offers none has no menu. */}
              {settings.effort !== null && (
                <Setting
                  title="Effort"
                  options={choices.efforts}
                  value={settings.effort}
                  disabled={changingModel || !effortsKnown(choices, settings.model)}
                  onChange={(effort) => onSettings({ ...settings, effort })}
                />
              )}
            </>
          )}
          {ring}
        </span>
      </div>
      {note !== null && <p className="px-1 text-xs text-muted-foreground">{note}</p>}
      {noMode && (
        <p className="px-1 text-xs text-destructive-foreground">
          This project allows no mode for {harnessLabel}: turns cannot start. Allow one on the
          project page.
        </p>
      )}
    </div>
  )
}

/** One setting's menu. A choice that cannot be picked is shown disabled with
 * the reason the session gave. */
function Setting({
  title,
  options,
  value,
  onChange,
  disabled = false,
}: {
  title: string
  options: readonly Choice[]
  value: string
  onChange: (id: string) => void
  disabled?: boolean
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={<Button variant="ghost" size="xs" title={title} disabled={disabled} />}
      >
        {labelOf(options, value)}
        <ChevronDown aria-hidden />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top">
        <DropdownMenuGroup>
          <DropdownMenuLabel>{title}</DropdownMenuLabel>
          <DropdownMenuRadioGroup value={value} onValueChange={(v: string) => onChange(v)}>
            {options.map((o) => (
              <DropdownMenuRadioItem key={o.id} value={o.id} disabled={!o.enabled}>
                <span className="flex flex-col">
                  <span>{o.label}</span>
                  {!o.enabled && o.reason !== null && (
                    <span className="text-xs text-muted-foreground">{o.reason}</span>
                  )}
                </span>
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
