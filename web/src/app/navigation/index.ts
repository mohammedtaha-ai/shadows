import { useNavigate, type RegisteredRouter, type InferFullSearchSchema } from '@tanstack/react-router'
type SectionId = NonNullable<InferFullSearchSchema<RegisteredRouter['routeTree']>['settings']>

export function useOpenSettings(): (section: SectionId, agent?: string) => void {
  const navigate = useNavigate()
  return (section, agent) => {
    void navigate({ to: '.', search: previous => ({ ...previous, settings: section, agent: section === 'agents' ? agent : undefined }) })
  }
}

export function useCloseSettings(): () => void {
  const navigate = useNavigate()
  return () => {
    void navigate({ to: '.', search: previous => ({ ...previous, settings: undefined, agent: undefined }) })
  }
}
