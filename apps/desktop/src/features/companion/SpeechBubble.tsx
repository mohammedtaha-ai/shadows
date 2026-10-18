import React from "react";
import type { CompanionMessage } from "./types.ts";
export function SpeechBubble({
  message,
}: {
  message: CompanionMessage | null;
}): React.JSX.Element | null {
  return message ? (
    <div
      className="aylul-bubble"
      role="status"
      aria-label="AYLUL message"
      aria-live="polite"
      dir="auto"
    >
      <bdi>{message.text}</bdi>
    </div>
  ) : null;
}
