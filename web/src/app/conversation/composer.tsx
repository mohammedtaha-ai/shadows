// One job: writing to the Planner — the prompt and the settings it runs with,
// Send, and Stop while a turn runs.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useSearch } from '@tanstack/react-router'
import { ArrowUp, Square } from 'lucide-react'
import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react'
import { type SessionChoices, type TurnSettings, changeEffort, changeModel, changeMode } from '@/api/sessions'
import { type Focus, queueMessage, startTurn, stopTurn, attachmentUrl } from '@/api/threads'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { queuedQuery } from '@/api/threads'
import { saveSessionAnswer } from '@/api/sessions'
import { Button } from '@/components/ui/button'
import type { SlashCommand } from '@/stream'
import {noteCompanionSend,useCompanionSendProblem} from '@/desktop'
import { tabId } from '@/stream'
import { ErrorLine } from '../error-line'
import { useOpenSettings } from '@/app/navigation'
import { uploadImages, retryImageUpload, type CarriedSend, type PendingImage } from './carried-send'
import { AYLUL_HARNESS } from './cli-picker'
import { ComposerBar } from './composer-bar'
import { FocusChip, type PointedTask } from './focus-chip'
import { type SessionCommand, sessionCommand } from './session-commands'
import { slashMatches } from './slash-filter'
import { SlashMenu } from './slash-menu'
import { afterOptions, effortsKnown, initialSettings, sendable, withModel } from './turn-settings'
import type { Turn } from './turn-state'
import { type SessionView } from './use-session'

/** Where Send goes: the open thread's next turn, or a draft's first message,
 * whose `draft` makes the thread first and answers the turn's operation. */
export type SendTo =
  | { threadId: string }
  | { draft: (commandId: string, text: string, settings: TurnSettings, images: PendingImage[], changed: () => void) => Promise<string>; planId?: string; selectableModels?: boolean }

const NONE: readonly SlashCommand[] = []

interface Send {
  images: PendingImage[]
  commandId: string
  text: string
  settings: TurnSettings
  pointed: PointedTask | null
  planId: string | undefined
}

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
  const filePicker = useRef<HTMLInputElement | null>(null)
  const [images, setImages] = useState<PendingImage[]>(carried?.images ?? [])
  const imagesChanged = () => setImages(now => [...now])
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

  // The command id belongs to a pending send: made when Send is
  // pressed with no pending send, reused by a retry of that same send, and
  // dropped when it succeeds or when the prompt or a setting changes, so a
  // changed request is a new command rather than a COMMAND_CONFLICT.
  const pending = useRef<Attempt | null>(carried?.attempt ?? null)

  const send = useMutation({
    mutationFn: async ({ commandId, text, settings, pointed, planId, images }: Send) =>{
      if('draft' in to)return to.draft(commandId,text,settings,images,imagesChanged)
      const attachments = await uploadImages(to.threadId, images, imagesChanged)
      await noteCompanionSend(to.threadId)
      return startTurn(to.threadId,commandId,text,settings,{
        focus:focusOf(pointed),plan:planId??null,clientTab:tabId(),attachments,
      })
    },
    onError: error => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') pending.current = null
    },
    onSuccess: (operationId, { pointed, images: sent }) => {
      setImages(now => now.filter(image => !sent.includes(image)))
      pending.current = null
      setContinuedPlan(undefined)
      setPrompt('')
      if (pointed !== null) onPointed(pointed)
      onStarted(operationId)
    },
  })

  // While a turn runs Enter queues the message with the settings the menus
  // hold; it is sent when the turn ends or by Send now.
  const queryClient = useQueryClient()
  const queue = useMutation({
    mutationFn: async ({commandId,text,settings,pointed,planId,images}:Send)=>{
      const attachments = await uploadImages(threadId as string, images, imagesChanged)
      await noteCompanionSend(threadId as string)
      return queueMessage(threadId as string,commandId,text,settings,{
        focus:focusOf(pointed),plan:planId??null,clientTab:tabId(),attachments,
      })
    },
    // Keep the complete message until Core accepts it, so a refusal cannot
    // lose its text while the images remain selected.
    onSuccess: (_answer, { images: sent }) => {
      setImages(now => now.filter(image => !sent.includes(image)))
      setPrompt('')
      pending.current = null
      void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId as string).queryKey })
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') pending.current = null
    },
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
    pending.current = null
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
  const busy = send.isPending || running !== null
  const { settings, note, choose: chooseLocal, refuse, keepEffort } = useTurnSettings(choices, busy, threadId !== null, () => {
    pending.current = null
  })
  const stop = useMutation({ mutationFn: stopTurn })

  // A lost answer may already have saved the mode. Keep its replay key until
  // Core answers, or the person chooses a different request.
  const modeAttempt = useRef<Attempt | null>(null)
  const switchMode = useMutation({
    mutationFn: ({ threadId, commandId, mode }: { threadId: string; commandId: string; mode: string }) =>
      changeMode(threadId, commandId, mode),
    onSuccess: async (answer, { threadId }) => {
      modeAttempt.current = null
      await saveSessionAnswer(queryClient, threadId, answer)
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon') modeAttempt.current = null
    },
  })
  const choose = (next: TurnSettings) => {
    if (threadId !== null && choices !== null && next.mode !== choices.current.mode) {
      pending.current = null
      modeAttempt.current = attemptFor(modeAttempt.current, { threadId, mode: next.mode })
      switchMode.mutate({ threadId, commandId: modeAttempt.current.commandId, mode: next.mode })
    } else {
      modeAttempt.current = null
      chooseLocal(next)
    }
  }

  // A picked model is set on the session at once, so its efforts
  // are known before Send. The answer is the session's new choices; a refusal
  // puts the session's model back and says why. A turn's session is not
  // changed while it runs: the change is asked once the turn has ended.
  const switchModel = useMutation({
    // A draft has no session to change: Send opens it.
    mutationFn: ({ threadId, id }: { threadId: string; id: string }) => changeModel(threadId, id),
    onSuccess: async (answer, { threadId, id }) => {
      await saveSessionAnswer(queryClient, threadId, answer)
      // Asked again it would be answered the same: say so rather than ask.
      if (answer.current.model !== id) refuse(`The session kept ${answer.current.model}`, answer)
      // The answer is the effort the move ended at (the remembered one);
      // a report streamed during the move may still carry the previous one.
      else keepEffort(answer)
    },
    onError: (error) => refuse(error.message),
  })
  const wanted = settings?.model
  const held = choices?.current.model
  const requestModel = switchModel.mutate
  const changing = switchModel.isPending
  useEffect(() => {
    if (busy || changing || wanted === undefined || held === undefined || wanted === held) return
    if (threadId !== null) requestModel({ threadId, id: wanted })
  }, [busy, changing, wanted, held, requestModel, threadId])

  // The same for a picked effort, once the session holds the chosen model:
  // the session then reports the effort the turn will run at. A refusal puts
  // the session's effort back, and the error line says why.
  const switchEffort = useMutation({
    mutationFn: ({ threadId, id }: { threadId: string; id: string }) => changeEffort(threadId, id),
    onSuccess: async (answer, { threadId, id }) => {
      await saveSessionAnswer(queryClient, threadId, answer)
      if (answer.current.effort !== id) keepEffort(answer)
    },
    onError: () => keepEffort(),
  })
  const wantedEffort = settings?.effort ?? null
  const heldEffort = choices?.current.effort ?? null
  const requestEffort = switchEffort.mutate
  const changingEffort = switchEffort.isPending
  useEffect(() => {
    if (busy || changing || changingEffort || threadId === null) return
    if (wanted !== held || wantedEffort === null || wantedEffort === heldEffort) return
    requestEffort({ threadId, id: wantedEffort })
  }, [busy, changing, changingEffort, wanted, held, wantedEffort, heldEffort, requestEffort, threadId])

  // "Stopping" until the durable ending arrives and `running` clears: the
  // stop call answering is not the turn ending. A stop this client saw fail
  // (PROCESS_TERMINATION_FAILED: the tree may still be running) offers Stop
  // again, even though the request is durable — asking again is safe, and a
  // button stuck on "Stopping…" over a live tree could never be retried. Only
  // a stop in flight disables it; a "Stopping…" learned from the daemon (after
  // a reload, say) stays pressable for the same reason.
  const mine = running !== null && stop.variables === running.id
  const failed = mine && stop.isError
  const stopping = running !== null && !failed && (running.stopRequested || mine)
  const stopLabel = failed ? 'Stop again' : stopping ? 'Stopping…' : 'Stop'

  // Send waits until the session holds the chosen model: only then are the
  // efforts it runs with that model's.
  const ready =
    known &&
    !changing &&
    !changingEffort &&
    !switchMode.isPending &&
    choices !== null &&
    settings !== null &&
    (('draft' in to && to.selectableModels === true) || effortsKnown(choices, settings.model)) &&
    sendable(choices, settings)

  const submit = () => {
    const command = sessionCommand(prompt, choices)
    if (command !== null) return act(command)
    const text = prompt.trim()
    if ((text === '' && images.length === 0) || images.some(image => image.error !== undefined) || !ready || settings === null || send.isPending || queue.isPending) return
    setSlashFromMenu(false)
    // The focus is part of the command: pointing elsewhere is a new one.
    setCarriedError(null)
    switchEffort.reset()
    pending.current = attemptFor(pending.current, { text, settings, ...(images.length === 0 ? {} : { attachments: images.map(image => image.key) }), focus: focusOf(pointed), ...(planId === undefined ? {} : { plan: planId }) })
    const args = { commandId: pending.current.commandId, text, settings, pointed, planId, images: [...images] }
    if (running !== null && threadId !== null) {
      queue.mutate(args)
    } else send.mutate(args)
  }

  const imageReason = choices === null ? 'The session is not ready yet.'
    : !choices.capabilities.image ? choices.capabilities.image_reason ?? 'This session does not support images.' : null
  const addImages = (files: File[]) => {
    if (imageReason !== null) {
      setCarriedError(new Error(imageReason))
      return
    }
    if (send.isPending || queue.isPending) return
    const added = files.map(file => ({ key: crypto.randomUUID(), file }))
    pending.current = null
    setImages(now => [...now, ...added])
    if (threadId !== null) void uploadImages(threadId, added, imagesChanged).catch(() => {})
  }
  const pickPhotos = () => {
    if (imageReason === null && !send.isPending && !queue.isPending) filePicker.current?.click()
    else if (imageReason !== null) setCarriedError(new Error(imageReason))
  }

  const error = send.error ?? queue.error ?? stop.error ?? switchMode.error ?? switchEffort.error ?? carriedError

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
          disabled={imageReason !== null || send.isPending || queue.isPending}
          onChange={e => { addImages(Array.from(e.target.files ?? [])); e.target.value = '' }} />
        {images.length > 0 && <ul aria-label="Images" className="flex flex-wrap gap-2">
          {images.map(image => <li key={image.key} className="max-w-40 space-y-1 text-xs">
            <ImagePreview image={image} threadId={threadId} />
            <p>{image.error?.message ?? (image.attachment ? 'Uploaded' : threadId === null ? 'Ready to upload on send' : 'Uploading...')}</p>
            {image.error && threadId !== null && <button type="button" disabled={send.isPending || queue.isPending}
              onClick={() => {
                void retryImageUpload(threadId, image, imagesChanged).catch(() => {})
              }}>Retry {image.file.name}</button>}
            <button type="button" aria-label={`Remove ${image.file.name}`} disabled={send.isPending || queue.isPending}
              onClick={() => { pending.current = null; setImages(now => now.filter(item => item !== image)) }}>Remove</button>
          </li>)}
        </ul>}

        {pointed !== null && <FocusChip pointed={pointed} onClear={() => onPointed(pointed)} />}
        <div className="flex items-end gap-2 rounded-xl border border-accent-line/40 bg-input-background p-2.5 shadow-xs transition-all duration-150 focus-within:border-accent-line focus-within:ring-2 focus-within:ring-accent-line/30 focus-within:shadow-md">
          <div className="relative flex-1">
            <textarea
              ref={messageBox}
              value={prompt}
              disabled={send.isPending || queue.isPending}
              onPaste={e => {
                const files = Array.from(e.clipboardData.files).filter(file => file.type.startsWith('image/'))
                if (files.length === 0) return
                e.preventDefault()
                addImages(files)
              }}
              onChange={(e) => {
                pending.current = null
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
              disabled={(prompt.trim() === '' && images.length === 0) || images.some(image => image.error !== undefined) || !ready || send.isPending || queue.isPending}
              size="icon"
              aria-label="Send"
              className="rounded-lg shadow-2xs transition-transform active:scale-95"
            >
              <ArrowUp />
            </Button>
          ) : (
            <Button
              variant="outline"
              onClick={() => stop.mutate(running.id)}
              disabled={stop.isPending}
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
          changingModel={changing || changingEffort || switchMode.isPending}
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

/** The settings the next turn runs with: the session's own at first, then the
 * person's, following the session's reports (`afterOptions`) as they arrive.
 * `changed` runs whenever the person changes one; `refuse` puts the model
 * back to the one the session holds (`session`, when it just answered) and
 * shows why; `keepEffort` puts the effort back to the session's. */
function useTurnSettings(
  choices: SessionChoices | null,
  busy: boolean,
  saved: boolean,
  changed: () => void,
): {
  settings: TurnSettings | null
  note: string | null
  choose: (next: TurnSettings) => void
  refuse: (why: string, session?: SessionChoices) => void
  keepEffort: (session?: SessionChoices) => void
} {
  const [held, setHeld] = useState<{
    choices: SessionChoices
    settings: TurnSettings
    note: string | null
  } | null>(null)

  // Adjusted during render, React's pattern for following a changed input:
  // the session's choices are replaced wholesale on every report.
  if (choices === null && held !== null) setHeld(null)
  if (choices !== null && held?.choices !== choices) {
    if (held === null) {
      setHeld({ choices, settings: saved ? choices.current : initialSettings(choices), note: null })
    } else {
      const next = afterOptions(held.choices, choices, held.settings, busy)
      // A note stays until the person changes a setting.
      setHeld({ choices, settings: saved ? { ...next.settings, mode: choices.current.mode } : next.settings, note: next.note ?? held.note })
    }
  }

  return {
    settings: held?.settings ?? null,
    note: held?.note ?? null,
    choose: (next) => {
      changed()
      setHeld((h) => (h === null ? h : { ...h, settings: next, note: null }))
    },
    refuse: (why, session) => {
      changed()
      setHeld((h) => {
        if (h === null) return h
        const back = (session ?? h.choices).current.model
        return { ...h, settings: withModel(h.choices, h.settings, back), note: why }
      })
    },
    keepEffort: (session) => {
      changed()
      setHeld((h) => {
        if (h === null) return h
        const effort = (session ?? h.choices).current.effort
        return { ...h, settings: { ...h.settings, effort } }
      })
    },
  }
}

/** What the turn sends: the task's id, never its number. */
function focusOf(pointed: PointedTask | null): Focus | null {
  if (pointed === null) return null
  return { workflow_id: pointed.workflowId, task_id: pointed.task.id, revision: pointed.revision }
}

function ImagePreview({ image, threadId }: { image: PendingImage; threadId: string | null }) {
  const file = image.file
  const saved = image.attachment?.id
  const preview = useCallback((node: HTMLImageElement | null) => {
    if (node === null) return
    if (saved !== undefined && threadId !== null) {
      node.src = attachmentUrl(threadId, saved)
      return
    }
    const local = URL.createObjectURL(file)
    node.src = local
    return () => URL.revokeObjectURL(local)
  }, [file, saved, threadId])
  return <img ref={preview} alt={file.name} className="size-24 rounded object-cover" />
}
