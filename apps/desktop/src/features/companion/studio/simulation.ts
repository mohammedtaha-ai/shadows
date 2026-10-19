import type { CompanionMessage, Expression, OutfitId } from "../types.ts";
export type SimulationClock = {
  set(callback: () => void, delayMs: number): number;
  clear(id: number): void;
};
export type DemoUpdate = {
  expression?: Expression;
  message?: CompanionMessage | null;
  outfit?: OutfitId;
  tone?: "poke" | "state" | "success";
};
export type Simulation = {
  run(kind: "success" | "blocked" | "sleep"): void;
  notify(): void;
  cancel(): void;
  dispose(): void;
};
export function createSimulation(
  clock: SimulationClock,
  emit: (update: DemoUpdate) => void,
): Simulation {
  let generation = 0,
    disposed = false,
    sequence = 0;
  const timers = new Set<number>();
  const cancel = () => {
    generation++;
    for (const timer of timers) clock.clear(timer);
    timers.clear();
  };
  const send = (
    expression: Expression,
    text: string,
    tone?: DemoUpdate["tone"],
  ) => {
    if (!disposed)
      emit({
        expression,
        message: { id: `preview-${++sequence}`, text },
        tone,
      });
  };
  const later = (delay: number, callback: () => void) => {
    const token = generation;
    const id = clock.set(() => {
      timers.delete(id);
      if (!disposed && token === generation) callback();
    }, delay);
    timers.add(id);
  };
  return {
    cancel,
    dispose() {
      cancel();
      disposed = true;
    },
    run(kind) {
      if (disposed) return;
      cancel();
      if (kind === "sleep") {
        send("sleeping", "Preview: sleeping");
        return;
      }
      send("listening", "Preview: listening");
      later(900, () => send("thinking", "Preview: thinking"));
      later(2100, () => send("working", "Preview: working"));
      later(4100, () =>
        send(
          kind === "blocked" ? "blocked" : "success",
          kind === "blocked"
            ? "Preview: intervention needed"
            : "Preview: completed",
          "success",
        ),
      );
      later(8500, () => emit({ message: null }));
    },
    notify() {
      if (disposed) return;
      cancel();
      send("listening", "Preview notification", "state");
      later(1800, () => send("happy", "Preview notification received"));
      later(5800, () => emit({ message: null }));
    },
  };
}
