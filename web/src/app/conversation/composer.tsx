// Compose the message box, its pickers and the delivery controls.

import { useSearch } from '@tanstack/react-router'
import { ArrowUp, Square } from 'lucide-react'
import { type ReactNode, useEffect, useRef, useState } from 'react'
import { type TurnSettings } from '@/api/sessions'
import { Button } from '@/components/ui/button'
import type { SlashCommand } from '@/stream'
import { useCompanionSendProblem } from '@/desktop'
import { ErrorLine } from '../error-line'
import { useOpenSettings } from '@/app/navigation'
import { type CarriedSend, type PendingImage } from './carried-send'
import { AYLUL_HARNESS } from './cli-picker'
import { ComposerBar } from './composer-bar'
import { FocusChip, type PointedTask } from './focus-chip'
import { type SessionCommand, sessionCommand } from './session-commands'
import { slashMatches } from './slash-filter'
import { SlashMenu } from './slash-menu'
import { effortsKnown, sendable, withModel } from './turn-settings'
import { useSessionSettings } from './use-session-settings'
import { useImages } from './use-images'
import { useSend } from './use-send'
import { useStop } from './use-stop'
import { PendingImages } from './pending-images'
import type { Turn } from './turn-state'
import { type SessionView } from './use-session'

/** Where Send goes: the open thread's next turn, or a draft's first message,
 * whose `draft` makes the thread first and answers the turn's operation. */
export type SendTo =
  | { threadId: string }
  | { draft: (commandId: string, text: string, settings: TurnSettings, images: PendingImage[], changed: () => void) => Promise<string>; planId?: string; selectableModels?: boolean }

const NONE: readonly SlashCommand[] = []

export function Composer({
  to,
  carried = null,
  harness,
  harnessLabel,
  session,
  running,
  directory,
  known,
  commands = NONE,
  ring,
  onStarted,
  pointed,
  onPointed,
  agentProfile,
}: {
  to: SendTo
  /** A draft's first message whose turn failed once its thread existed: its
   * text, command and error, shown here as the draft showed them. */
  carried?: CarriedSend | null
  /** The thread's harness kind. */
  harness: string
  harnessLabel: string
  session: SessionView
  running: Turn | null
  directory: string | null | undefined
  /** Whether a turn is running is known yet; Send waits for it. */
  known: boolean
  /** The harness's `/` list; none opens no menu. */
  commands?: readonly SlashCommand[]
  ring?: ReactNode
  /** The start route answered: the turn exists before its events arrive. */
  onStarted: (operationId: string) => void
  /** The task the person points at, sent with the next turn. */
  pointed: PointedTask | null
  /** Clears the chip: pressed ×, or the turn that carried it started. */
  onPointed: (done: PointedTask) => void
  /** The live AYLUL agent whose settings belong to this conversation. */
  agentProfile?: string
}) {
  const messageBox = useRef<HTMLTextAreaElement | null>(null)
  const returnFromPlugins = useRef(false)
  const { settings: settingsSection } = useSearch({ from: '__root__' })
  const openSettings = useOpenSettings()
  useEffect(() => {
    if (settingsSection !== undefined || !returnFromPlugins.current) return
    let frame: number | undefined
    // The dialog restores focus on unmount, after its exit animation. Waiting for
    // removal keeps that restoration from taking focus back from the message box.
    const observer = new MutationObserver(restore)
    function restore() {
      if (document.querySelector('[role="dialog"]') !== null) return
      observer.disconnect()
      frame = requestAnimationFrame(() => {
        returnFromPlugins.current = false
        messageBox.current?.focus()
      })
    }
    observer.observe(document.body, { childList: true, subtree: true })
    restore()
    return () => {
      observer.disconnect()
      if (frame !== undefined) cancelAnimationFrame(frame)
    }
  }, [settingsSection])
  const threadId = 'threadId' in to ? to.threadId : null
  const companionProblem=useCompanionSendProblem(threadId)
  const [prompt, setPrompt] = useState(carried?.text ?? '')
  const [carriedError, setCarriedError] = useState(carried?.error ?? null)
  const [continuedPlan, setContinuedPlan] = useState(carried?.planId)
  const planId = 'draft' in to ? to.planId : continuedPlan

  // Image callbacks run on delivery, after the selection hook has been composed.
  const { deliver, clearAttempt, sending, inFlight, error: sendError } = useSend({
    to, threadId, running, initialAttempt: carried?.attempt,
    imagesChanged: () => imagesChanged(),
    dropImages: sent => dropImages(sent),
    clearPrompt: () => setPrompt(''),
    clearContinuedPlan: () => setContinuedPlan(undefined),
    onPointed, onStarted,
  })

  // The `/` menu: open while the whole text is `/name`; Escape closes
  // it on one text, and any change reopens it. A picked entry's hint shows
  // after the caret until the text changes.
  const [highlighted, setHighlighted] = useState(0)
  const [dismissed, setDismissed] = useState<string | null>(null)
  const [hint, setHint] = useState<{ text: string; hint: string } | null>(null)
  // `model` and `effort` act through the pickers, never as text.
  const [opened, setOpened] = useState<'model' | 'effort' | null>(null)
  const [slashFromMenu, setSlashFromMenu] = useState(false)
  const matches = slashMatches(commands, slashFromMenu ? '/' : prompt)
  const menu = matches !== null && matches.length > 0 && (slashFromMenu || dismissed !== prompt)
    ? matches : null
  const at = Math.min(highlighted, (menu?.length ?? 1) - 1)
  const pick = (command: SlashCommand) => {
    setSlashFromMenu(false)
    setHighlighted(0)
    if (command.name === 'model' || command.name === 'effort') {
      setPrompt('')
      setHint(null)
      setOpened(command.name)
      return
    }
    const text = `/${command.name} `
    clearAttempt()
    setPrompt(text)
    setHint(command.hint === null ? null : { text, hint: command.hint })
  }
  const act = (command: SessionCommand) => {
    if (command.kind === 'unknown') return setCarriedError(new Error(command.message))
    // No pickers without a session, and the effort picker waits, as it does
    // for the menu, until the session holds the chosen model.
    if (choices === null || settings === null) {
      return setCarriedError(new Error('The session is not ready yet.'))
    }
    if (command.setting === 'effort' && !effortsKnown(choices, settings.model)) {
      return setCarriedError(new Error('Efforts are known once the session holds the chosen model.'))
    }
    setCarriedError(null)
    setPrompt('')
    if (command.kind === 'open') return setOpened(command.setting)
    if (command.setting === 'model') choose(withModel(choices, settings, command.id))
    else choose({ ...settings, effort: command.id })
  }

  const choices = session.state === 'ready' ? session.choices : null
  const { images, imageReason, filePicker, add: addImages, pick: pickPhotos, retry, remove, drop: dropImages, imagesChanged } = useImages(
    choices, threadId, inFlight, clearAttempt, setCarriedError, carried?.images,
  )
  const busy = sending || running !== null
  const { settings, note, choose, switching, error: settingsError, resetError: resetSettingsError } = useSessionSettings(choices, busy, threadId, clearAttempt)
  const { stop, isPending: stopping, error: stopError, label: stopLabel } = useStop(running)

  // Send waits until the session holds the chosen model: only then are the
  // efforts it runs with that model's.
  const ready =
    known &&
    !switching &&
    choices !== null &&
    settings !== null &&
    (('draft' in to && to.selectableModels === true) || effortsKnown(choices, settings.model)) &&
    sendable(choices, settings)

  const submit = () => {
    const command = sessionCommand(prompt, choices)
    if (command !== null) return act(command)
    const text = prompt.trim()
    if ((text === '' && images.length === 0) || images.some(image => image.error !== undefined) || !ready || settings === null || inFlight) return
    setSlashFromMenu(false)
    setCarriedError(null)
    resetSettingsError()
    deliver({ text, settings, pointed, planId, images })
  }

  const error = sendError ?? stopError ?? settingsError ?? carriedError

  return (
    <div className="border-t border-border bg-background/95 px-6 pt-3 pb-4 backdrop-blur-md">
      <div className="mx-auto max-w-3xl space-y-2"
        onKeyDown={e => {
          if (e.ctrlKey && e.key.toLowerCase() === 'u') {
            e.preventDefault()
            pickPhotos()
          }
        }}
        onDragOver={e => { if (e.dataTransfer.types.includes('Files')) e.preventDefault() }}
        onDrop={e => {
          if (e.dataTransfer.files.length === 0) return
          e.preventDefault()
          addImages(Array.from(e.dataTransfer.files))
        }}
      >
        <input ref={filePicker} type="file" accept="image/*" multiple hidden aria-label="Add photos"
          disabled={imageReason !== null || inFlight}
          onChange={e => { addImages(Array.from(e.target.files ?? [])); e.target.value = '' }} />
        <PendingImages images={images} threadId={threadId} inFlight={inFlight} onRetry={retry} onRemove={remove} />

        {pointed !== null && <FocusChip pointed={pointed} onClear={() => onPointed(pointed)} />}
        <div className="flex items-end gap-2 rounded-xl border border-accent-line/40 bg-input-background p-2.5 shadow-xs transition-all duration-150 focus-within:border-accent-line focus-within:ring-2 focus-within:ring-accent-line/30 focus-within:shadow-md">
          <div className="relative flex-1">
            <textarea
              ref={messageBox}
              value={prompt}
              disabled={inFlight}
              onPaste={e => {
                const files = Array.from(e.clipboardData.files).filter(file => file.type.startsWith('image/'))
                if (files.length === 0) return
                e.preventDefault()
                addImages(files)
              }}
              onChange={(e) => {
                clearAttempt()
                setSlashFromMenu(false)
                setPrompt(e.target.value)
                // Any change reopens a dismissed menu if the text still qualifies.
                setHighlighted(0)
                setDismissed(null)
                if (hint?.text !== e.target.value) setHint(null)
              }}
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing) return
                // Shift+Enter inserts a newline, which closes the menu.
                if (menu !== null && !(e.key === 'Enter' && e.shiftKey)) {
                  const keys: Record<string, () => void> = {
                    ArrowDown: () => setHighlighted((at + 1) % menu.length),
                    ArrowUp: () => setHighlighted((at - 1 + menu.length) % menu.length),
                    Tab: () => pick(menu[at]),
                    Enter: () => pick(menu[at]),
                    Escape: () => {
                      setSlashFromMenu(false)
                      setDismissed(prompt)
                    },
                  }
                  const key = keys[e.key]
                  if (key !== undefined) {
                    e.preventDefault()
                    key()
                    return
                  }
                }
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault()
                  submit()
                }
              }}
              rows={2}
              placeholder={harness === AYLUL_HARNESS ? 'Ask AYLUL…' : 'Ask the Planner…'}
              aria-label="Message"
              className="max-h-48 min-h-10 w-full resize-none bg-transparent px-2 py-1 text-sm text-foreground outline-none placeholder:text-faint-foreground"
            />
            {menu !== null && (
              <SlashMenu items={menu} highlighted={at} onPick={pick} onHighlight={setHighlighted} />
            )}
            {hint !== null && hint.text === prompt && (
              <div
                aria-hidden
                className="pointer-events-none absolute inset-0 overflow-hidden px-2 py-1 text-sm break-words whitespace-pre-wrap"
              >
                <span className="invisible">{prompt}</span>
                <span className="text-faint-foreground">{hint.hint}</span>
              </div>
            )}
          </div>
          {running === null ? (
            <Button
              onClick={submit}
              disabled={(prompt.trim() === '' && images.length === 0) || images.some(image => image.error !== undefined) || !ready || inFlight}
              size="icon"
              aria-label="Send"
              className="rounded-lg shadow-2xs transition-transform active:scale-95"
            >
              <ArrowUp />
            </Button>
          ) : (
            <Button
              variant="outline"
              onClick={() => stop(running.id)}
              disabled={stopping}
              className="border-destructive-border text-destructive-foreground shadow-2xs transition-all hover:bg-destructive/10 hover:text-destructive-foreground"
            >
              <Square className="size-3 fill-current" />
              {stopLabel}
            </Button>
          )}
        </div>
        <ComposerBar
          harnessLabel={harnessLabel}
          session={session}
          settings={settings}
          onSettings={choose}
          changingModel={switching}
          sessionless={threadId === null}
          selectableModels={'draft' in to && to.selectableModels === true}
          harness={harness}
          directory={directory}
          note={note}
          ring={ring}
          opened={opened}
          onOpened={setOpened}
          messageBox={messageBox}
          imageReason={imageReason}
          onPhotos={pickPhotos}
          onSlashCommands={() => {
            setHighlighted(0)
            setDismissed(null)
            setHint(null)
            setSlashFromMenu(true)
          }}
          onPlugins={harness === AYLUL_HARNESS && agentProfile !== undefined ? () => {
            returnFromPlugins.current = true
            openSettings('agents', agentProfile)
          } : undefined}
        />
        {error !== null && <ErrorLine error={error} />}
        {companionProblem&&<p role="status" className="text-xs text-muted-foreground">{companionProblem}</p>}
      </div>
    </div>
  )
}
