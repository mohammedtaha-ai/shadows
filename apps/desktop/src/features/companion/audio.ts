export type ToneController = {
  enable(): Promise<void>;
  play(kind: "poke" | "state" | "success"): void;
  disable(): void;
  dispose(): void;
};
export function createTones(): ToneController {
  let context: AudioContext | null = null,
    enabled = false,
    disposed = false;
  const oscillators = new Set<OscillatorNode>();
  const stop = () => {
    for (const node of oscillators) {
      node.stop();
      node.disconnect();
    }
    oscillators.clear();
  };
  return {
    async enable() {
      if (disposed) return;
      if (!context) context = new AudioContext();
      await context.resume();
      if (context.state !== "running") throw new Error("Audio unavailable");
      if (!disposed) enabled = true;
    },
    play(kind) {
      if (!enabled || !context || disposed || context.state !== "running")
        return;
      const oscillator = context.createOscillator(),
        gain = context.createGain();
      oscillator.frequency.value =
        kind === "success" ? 880 : kind === "poke" ? 660 : 440;
      gain.gain.setValueAtTime(0.04, context.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + 0.15);
      oscillator.connect(gain);
      gain.connect(context.destination);
      oscillators.add(oscillator);
      oscillator.onended = () => {
        oscillators.delete(oscillator);
        oscillator.disconnect();
        gain.disconnect();
      };
      oscillator.start();
      oscillator.stop(context.currentTime + 0.16);
    },
    disable() {
      enabled = false;
      stop();
    },
    dispose() {
      disposed = true;
      enabled = false;
      stop();
      void context?.close().catch(() => {});
      context = null;
    },
  };
}
