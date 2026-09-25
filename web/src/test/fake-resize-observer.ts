// One job: a ResizeObserver a test resizes by hand. happy-dom's observes
// nothing and lays nothing out, so a test that needs "this element changed
// size" says so through `resize`. Test-only.

interface Observation {
  readonly element: Element
  readonly observer: FakeResizeObserver
}

const observations: Observation[] = []

export class FakeResizeObserver {
  readonly #callback: ResizeObserverCallback

  constructor(callback: ResizeObserverCallback) {
    this.#callback = callback
  }

  observe(element: Element): void {
    observations.push({ element, observer: this })
  }

  unobserve(element: Element): void {
    this.#forget((o) => o.element === element && o.observer === this)
  }

  disconnect(): void {
    this.#forget((o) => o.observer === this)
  }

  notify(element: Element, width: number, height: number): void {
    const entry = { target: element, contentRect: { width, height } } as unknown as ResizeObserverEntry
    this.#callback([entry], this as unknown as ResizeObserver)
  }

  #forget(match: (o: Observation) => boolean): void {
    for (let i = observations.length - 1; i >= 0; i -= 1) {
      const o = observations[i]
      if (o !== undefined && match(o)) observations.splice(i, 1)
    }
  }
}

/** Tells every observer of `element` that it is now `width` × `height`. */
export function resize(element: Element, width: number, height: number): void {
  for (const o of observations.filter((o) => o.element === element)) {
    o.observer.notify(element, width, height)
  }
}
