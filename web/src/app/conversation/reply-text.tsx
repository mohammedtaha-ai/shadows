// One job: rendering the Planner's markdown in the reply voice, streamed or
// settled.

import { code } from '@streamdown/code'
import { Streamdown, defaultRehypePlugins } from 'streamdown'
import { rehypeDirAuto } from './bidi'

const plugins = { code }
// Streamdown's own plugins first (raw HTML, sanitising, hardening), then each
// block's direction on what they leave.
const rehypePlugins = [...Object.values(defaultRehypePlugins), rehypeDirAuto]

/** While `live`, the text is still arriving: incomplete markdown is closed as
 * it streams and a purple cursor follows the last word. */
export function ReplyText({ text, live = false }: { text: string; live?: boolean }) {
  return (
    <Streamdown
      plugins={plugins}
      rehypePlugins={rehypePlugins}
      isAnimating={live}
      caret={live ? 'block' : undefined}
      className="reply-bidi font-serif text-[15px] leading-7 text-foreground [&>*:last-child]:after:ms-0.5 [&>*:last-child]:after:text-accent-line [&>*:last-child]:after:motion-safe:animate-pulse"
    >
      {text}
    </Streamdown>
  )
}
