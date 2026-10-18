export function seededRandom(seed = 12) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}
export function fakeFrames() {
  let next = 1;
  const pending = new Map<number, (time: number) => void>();
  return {
    request(callback: (time: number) => void) {
      const id = next++;
      pending.set(id, callback);
      return id;
    },
    cancel(id: number) {
      pending.delete(id);
    },
    tick(time: number) {
      const callbacks = [...pending.values()];
      pending.clear();
      callbacks.forEach((callback) => callback(time));
    },
    get pending() {
      return pending.size;
    },
  };
}
