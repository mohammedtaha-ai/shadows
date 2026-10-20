import React from "react";
import type { CompanionMessage } from "./types.ts";
export function SpeechBubble({
  message,
  compact=false,
}: {
  message: CompanionMessage | null;
  compact?:boolean;
}): React.JSX.Element | null {
  return message ? (
    <div
      className={compact?"aylul-bubble aylul-bubble-compact":"aylul-bubble"}
      role="status"
      aria-label="AYLUL message"
      aria-live="polite"
      dir="auto"
    >
      <bdi>{message.text}</bdi>
    </div>
  ) : null;
}
