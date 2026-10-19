import type { CompanionMessage } from "../types.ts";
export type MessageSlot = {
  message: CompanionMessage | null;
  deadline: number;
  id: string | null;
};
export function createMessageSlot(): MessageSlot {
  return { message: null, deadline: 0, id: null };
}
export function receiveMessage(
  slot: MessageSlot,
  message: CompanionMessage | null,
  nowMs: number,
): void {
  if (message === null) {
    slot.message = null;
    slot.id = null;
    slot.deadline = 0;
    return;
  }
  if (slot.id === message.id) return;
  const duration =
    message.durationMs !== undefined &&
    Number.isFinite(message.durationMs) &&
    message.durationMs >= 0
      ? message.durationMs
      : 4000;
  slot.message = message;
  slot.id = message.id;
  slot.deadline = nowMs + duration;
}
export function visibleMessage(
  slot: MessageSlot,
  nowMs: number,
): CompanionMessage | null {
  return nowMs < slot.deadline ? slot.message : null;
}
