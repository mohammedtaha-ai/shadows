// One job: the main pane when no thread is open.

export function Home() {
  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <p className="text-sm text-faint-foreground">No thread open.</p>
    </div>
  )
}
