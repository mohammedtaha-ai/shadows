export type FrameClock = {
  request(callback: (nowMs: number) => void): number;
  cancel(id: number): void;
};
export type FrameLoop = {
  start(): void;
  stop(): void;
  invalidate(): void;
  dispose(): void;
};
export function createFrameLoop(
  clock: FrameClock,
  draw: (seconds: number) => void,
): FrameLoop {
  let pending: number | null = null,
    running = false,
    disposed = false,
    last: number | null = null,
    generation = 0;
  const cancel = () => {
    generation++;
    if (pending !== null) clock.cancel(pending);
    pending = null;
    last = null;
  };
  const request = () => {
    if (disposed || pending !== null) return;
    const token = generation;
    pending = clock.request((now) => {
      if (disposed || token !== generation) return;
      pending = null;
      const seconds =
        running && last !== null
          ? Math.min(0.05, Math.max(0, (now - last) / 1000))
          : 0;
      last = now;
      draw(seconds);
      if (running) request();
    });
  };
  return {
    start() {
      if (disposed || running) return;
      running = true;
      last = null;
      request();
    },
    stop() {
      running = false;
      cancel();
    },
    invalidate() {
      request();
    },
    dispose() {
      running = false;
      disposed = true;
      cancel();
    },
  };
}
