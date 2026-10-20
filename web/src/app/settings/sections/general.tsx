import { ActiveProjects } from '../active-projects'

export function GeneralSection() {
  return (
    <section className="space-y-4 rounded-lg border border-border p-4" aria-label="Projects">
      <h2 className="text-sm font-medium">Projects</h2>
      <ActiveProjects />
    </section>
  )
}
