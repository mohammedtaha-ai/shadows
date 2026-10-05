// One job: what the main pane shows while a lazily loaded page arrives.
// Without it a direct visit or a reload showed an empty pane until the page's
// chunk loaded.

export function PagePending() {
  return (
    <div className="flex flex-1 items-center justify-center p-8 text-sm text-faint-foreground">
      Loading…
    </div>
  )
}
