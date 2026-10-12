// Shared display names of the backend-owned vision fields.
import type { VisionField } from '@/api/design'
export const VISION_FIELDS: VisionField[] = ['purpose', 'users', 'goals', 'boundaries', 'technical_direction']
const LABELS: Record<VisionField, string> = {
  purpose: 'Purpose', users: 'Users', goals: 'Goals', boundaries: 'Boundaries', technical_direction: 'Technical direction',
}
export const fieldLabel = (field: VisionField) => LABELS[field]
