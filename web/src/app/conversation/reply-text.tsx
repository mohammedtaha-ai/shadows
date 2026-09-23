// One job: rendering the Planner's markdown in the reply voice, streamed or
// settled.

import { code } from '@streamdown/code'
import { Streamdown } from 'streamdown'

const plugins = { code }

/** While `live`, the text is still arriving: incomplete markdown is closed as
 * it streams and a purple cursor follows the last word. */
export function ReplyText({ text, live = false }: { text: string; live?: boolean }) {
  return (
    <Streamdown
      plugins={plugins}
      isAnimating={live}
      caret={live ? 'block' : undefined}
      className="font-serif text-[15px] leading-7 text-foreground [&>*:last-child]:after:ml-0.5 [&>*:last-child]:after:text-accent-line [&>*:last-child]:after:motion-safe:animate-pulse"
    >
      {text}
    </Streamdown>
  )
}
