// One job: drawing the `/` menu (spec §21.4): the entries, the highlighted
// one, and its description beside the list. Keys are the composer's.

import { useEffect, useRef } from 'react'
import type { SlashCommand } from '@/stream/frames'

export function SlashMenu({
  items,
  highlighted,
  onPick,
  onHighlight,
}: {
  items: readonly SlashCommand[]
  highlighted: number
  onPick: (command: SlashCommand) => void
  onHighlight: (index: number) => void
}) {
  const active = useRef<HTMLLIElement | null>(null)
  useEffect(() => active.current?.scrollIntoView({ block: 'nearest' }), [highlighted])
  const current = items[highlighted]
  return (
    <div className="absolute bottom-full left-0 mb-2 flex items-start gap-2">
      <ul
        role="listbox"
        aria-label="Commands"
        dir="ltr"
        className="max-h-80 w-72 overflow-y-auto rounded-lg border border-border bg-popover p-1 text-sm shadow-md"
      >
        {items.map((command, i) => (
          <li
            key={command.name}
            ref={i === highlighted ? active : undefined}
            role="option"
            aria-selected={i === highlighted}
            data-name={command.name}
            onMouseEnter={() => onHighlight(i)}
            onMouseDown={(e) => {
              e.preventDefault() // keep the textarea's focus
              onPick(command)
            }}
            className={`cursor-pointer truncate rounded px-2 py-1 ${i === highlighted ? 'bg-accent text-accent-foreground' : 'text-foreground'}`}
          >
            {command.name}
          </li>
        ))}
      </ul>
      {current !== undefined && current.description !== '' && (
        <p
          dir="auto"
          className="max-w-xs rounded-lg border border-border bg-popover p-2 text-xs text-muted-foreground shadow-md"
        >
          {current.description}
        </p>
      )}
    </div>
  )
}
